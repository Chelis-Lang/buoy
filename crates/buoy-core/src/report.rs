//! Complete orthogonal v2 report facts and authoritative binary report identity.
//!
//! Constructors intentionally consume complete input collections so callers
//! cannot mutate retained semantic state after validation. Every constructor
//! returns the same canonical encoding failures documented by [`EncodingError`].

#![allow(clippy::missing_errors_doc, clippy::needless_pass_by_value)]

use crate::algorithms::{
    Diagnostic, DispositionProtocolContext, Impact, ProofInvalidationContext,
    disposition_protocol_closure, proof_invalidation_reasons,
};
use crate::identifier::StableId;
use crate::model::{
    ApprovalBinding, ArtifactBinding, ArtifactIndex, AssuranceClass, ChangeDispositionKind,
    ChangeManifest, DebtLifecycle, EnforcementMode, EvidenceRole, LifecycleState,
    NormalizedVerdict, ObjectKind, ObjectRef, ObligationKind, ObligationRef, ProofRecord,
    SpecGraph, SurfaceDispositionKind, TypedPayload, Verdict, WaiverLifecycle,
};
use crate::value::{
    CanonicalValue, DecodeBudget, DecodingError, EncodingError, SemanticDigest, semantic_digest,
    semantic_digest_from_encoded_record,
};
use alloc::boxed::Box;
use alloc::string::{String, ToString};
use alloc::vec::Vec;

/// Validity fact identity.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(u16)]
pub enum Validity {
    /// Valid under the selected configuration.
    Valid = 1,
    /// Invalid under the selected configuration.
    Invalid = 2,
}

/// Completeness fact identity.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(u16)]
pub enum Completeness {
    /// Complete finite set.
    Complete = 1,
    /// Valid but incomplete finite set.
    Incomplete = 2,
    /// Input was invalid before completeness could hold.
    Invalid = 3,
}

/// Freshness fact identity.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(u16)]
pub enum Freshness {
    /// Every required binding is current.
    Fresh = 1,
    /// At least one required binding is stale.
    Stale = 2,
    /// Binding shape is invalid.
    Invalid = 3,
}

/// Obligation satisfaction identity.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(u16)]
pub enum Satisfaction {
    /// Technical obligation is satisfied.
    Satisfied = 1,
    /// Technical obligation is unsatisfied.
    Unsatisfied = 2,
    /// Policy scope does not apply.
    NotApplicable = 3,
}

/// Oracle/verifier selection identity.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(u16)]
pub enum Selection {
    /// Selected by exact configuration.
    Selected = 1,
    /// Discovered but not selected.
    NotSelected = 2,
    /// Selection declaration is invalid.
    Invalid = 3,
}

/// Disposition closure identity.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(u16)]
pub enum Closure {
    /// Exactly one applicable disposition exists.
    Closed = 1,
    /// Disposition is absent, duplicate, or stale.
    Open = 2,
    /// Impact/disposition input is invalid.
    Invalid = 3,
}

/// Derivation consistency identity.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(u16)]
pub enum Consistency {
    /// Consistency oracle was not run.
    NotRun = 1,
    /// Consistency oracle established the declared relation.
    Consistent = 2,
    /// Consistency oracle refuted the declared relation.
    Inconsistent = 3,
    /// Consistency execution or normalization errored.
    Error = 4,
    /// Consistency oracle timed out.
    Timeout = 5,
}

/// Canonical diagnostic severity.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(u16)]
pub enum DiagnosticSeverity {
    /// Visible but nonblocking under current policy.
    Advisory = 1,
    /// Blocks current selected acceptance.
    Blocking = 2,
}

/// Direct/transitive impact identity.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(u16)]
pub enum ImpactDistance {
    /// Object bytes/key changed directly.
    Direct = 1,
    /// Object depends transitively on a direct change.
    Transitive = 2,
}

/// Unified report-only governance lifecycle identity.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(u16)]
pub enum GovernanceLifecycle {
    /// Current active record.
    Active = 1,
    /// Evidence-backed discharge.
    Discharged = 2,
    /// Explicit revocation.
    Revoked = 3,
    /// Terminal supersession.
    Superseded = 4,
}

fn enum_value<T>(value: T) -> CanonicalValue
where
    T: Into<u16>,
{
    CanonicalValue::Enum(value.into(), None)
}

macro_rules! enum_into_u16 {
    ($($name:ty),+ $(,)?) => {
        $(impl From<$name> for u16 {
            fn from(value: $name) -> Self { value as Self }
        })+
    };
}

enum_into_u16!(
    Validity,
    Completeness,
    Freshness,
    Satisfaction,
    Selection,
    Closure,
    Consistency,
    DiagnosticSeverity,
    ImpactDistance,
    GovernanceLifecycle,
    LifecycleState,
    ObjectKind,
    ObligationKind,
    EvidenceRole,
    EnforcementMode,
    AssuranceClass,
    Verdict,
    ChangeDispositionKind,
    SurfaceDispositionKind,
);

fn object_ref(reference: &ObjectRef) -> CanonicalValue {
    CanonicalValue::Record(alloc::vec![
        (1, enum_value(reference.kind())),
        (2, CanonicalValue::Text(reference.id().as_str().to_string())),
        (3, CanonicalValue::Digest(reference.digest())),
    ])
}

fn object_ref_option(reference: Option<&ObjectRef>) -> CanonicalValue {
    reference.map_or(CanonicalValue::None, |reference| {
        CanonicalValue::Some(Box::new(object_ref(reference)))
    })
}

fn digest_option(digest: Option<SemanticDigest>) -> CanonicalValue {
    digest.map_or(CanonicalValue::None, |digest| {
        CanonicalValue::Some(Box::new(CanonicalValue::Digest(digest)))
    })
}

fn text_set(values: Vec<String>) -> CanonicalValue {
    CanonicalValue::Set(values.into_iter().map(CanonicalValue::Text).collect())
}

fn digest_set(values: Vec<SemanticDigest>) -> CanonicalValue {
    CanonicalValue::Set(values.into_iter().map(CanonicalValue::Digest).collect())
}

fn object_ref_set(values: Vec<ObjectRef>) -> CanonicalValue {
    CanonicalValue::Set(values.iter().map(object_ref).collect())
}

fn payload(value: &TypedPayload) -> CanonicalValue {
    CanonicalValue::Record(alloc::vec![
        (1, CanonicalValue::Text(value.schema().to_string())),
        (2, CanonicalValue::Bytes(value.bytes().to_vec())),
    ])
}

fn payload_set(values: Vec<TypedPayload>) -> CanonicalValue {
    CanonicalValue::Set(values.iter().map(payload).collect())
}

fn obligation(value: &ObligationRef) -> CanonicalValue {
    CanonicalValue::Record(alloc::vec![
        (
            1,
            CanonicalValue::Text(value.policy_id().as_str().to_string())
        ),
        (2, enum_value(value.kind())),
        (3, object_ref(value.subject())),
        (
            4,
            value.required_role().map_or(CanonicalValue::None, |role| {
                CanonicalValue::Some(Box::new(enum_value(role)))
            }),
        ),
    ])
}

fn obligation_set(values: Vec<ObligationRef>) -> CanonicalValue {
    CanonicalValue::Set(values.iter().map(obligation).collect())
}

fn artifact(value: &ArtifactBinding) -> CanonicalValue {
    match value {
        ArtifactBinding::Object(reference) => CanonicalValue::Record(alloc::vec![
            (1, enum_value(1_u16)),
            (2, CanonicalValue::Some(Box::new(object_ref(reference)))),
            (3, CanonicalValue::None),
            (4, CanonicalValue::Digest(reference.digest())),
            (5, CanonicalValue::None),
            (6, CanonicalValue::None),
            (7, CanonicalValue::None),
        ]),
        ArtifactBinding::RepositoryArtifact(binding) => CanonicalValue::Record(alloc::vec![
            (1, enum_value(2_u16)),
            (2, CanonicalValue::None),
            (
                3,
                CanonicalValue::Some(Box::new(CanonicalValue::Text(
                    binding.path().as_str().to_string(),
                ))),
            ),
            (4, CanonicalValue::Digest(binding.digest())),
            (
                5,
                CanonicalValue::Some(Box::new(CanonicalValue::Text(
                    binding.raw_domain().to_string(),
                ))),
            ),
            (
                6,
                CanonicalValue::Some(Box::new(CanonicalValue::Text(
                    binding.media_type().to_string(),
                ))),
            ),
            (
                7,
                CanonicalValue::Some(Box::new(CanonicalValue::Text(
                    binding.representation().to_string(),
                ))),
            ),
        ]),
    }
}

fn artifact_set(values: Vec<ArtifactBinding>) -> CanonicalValue {
    CanonicalValue::Set(values.iter().map(artifact).collect())
}

fn checked(value: CanonicalValue) -> Result<CanonicalValue, EncodingError> {
    value.encode()?;
    Ok(value)
}

macro_rules! fact_value {
    ($name:ident, $doc:literal) => {
        #[doc = $doc]
        #[derive(Clone, Debug, Eq, PartialEq)]
        pub struct $name(CanonicalValue);
        impl $name {
            fn value(&self) -> CanonicalValue {
                self.0.clone()
            }
        }
    };
}

fact_value!(AuthorityFact, "One complete orthogonal authority fact.");
fact_value!(SurfaceFact, "One complete orthogonal surface fact.");
fact_value!(
    RegistrationFact,
    "One complete orthogonal registration fact."
);
fact_value!(PolicyFact, "One complete orthogonal policy fact.");
fact_value!(
    FreshnessFact,
    "One complete orthogonal binding-freshness fact."
);
fact_value!(
    DebtWaiverFact,
    "One complete orthogonal debt or waiver fact."
);
fact_value!(DerivationFact, "One complete orthogonal derivation fact.");
fact_value!(ProofFact, "One complete orthogonal proof fact.");
fact_value!(
    EvidenceFact,
    "One complete orthogonal execution evidence fact."
);
fact_value!(
    DispositionClosureFact,
    "One complete orthogonal impact-disposition fact."
);
fact_value!(
    CanonicalDiagnostic,
    "One complete canonical diagnostic fact."
);
impl AuthorityFact {
    /// Constructs a complete authority fact.
    pub fn new(
        subject: ObjectRef,
        validity: Validity,
        lifecycle: LifecycleState,
        authority_source_count: u64,
        approval_validity: Validity,
        diagnostic_codes: Vec<String>,
    ) -> Result<Self, EncodingError> {
        checked(CanonicalValue::Record(alloc::vec![
            (1, object_ref(&subject)),
            (2, enum_value(validity)),
            (3, enum_value(lifecycle)),
            (4, CanonicalValue::U64(authority_source_count)),
            (5, enum_value(approval_validity)),
            (6, text_set(diagnostic_codes)),
        ]))
        .map(Self)
    }
}

impl SurfaceFact {
    /// Constructs a complete finite-surface fact.
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        surface: ObjectRef,
        domain_digest: SemanticDigest,
        completeness: Completeness,
        expected_count: u64,
        actual_count: u64,
        missing_keys: Vec<TypedPayload>,
        duplicate_keys: Vec<TypedPayload>,
        unknown_keys: Vec<TypedPayload>,
        unauthorized_keys: Vec<TypedPayload>,
        diagnostic_codes: Vec<String>,
    ) -> Result<Self, EncodingError> {
        checked(CanonicalValue::Record(alloc::vec![
            (1, object_ref(&surface)),
            (2, CanonicalValue::Digest(domain_digest)),
            (3, enum_value(completeness)),
            (4, CanonicalValue::U64(expected_count)),
            (5, CanonicalValue::U64(actual_count)),
            (6, payload_set(missing_keys)),
            (7, payload_set(duplicate_keys)),
            (8, payload_set(unknown_keys)),
            (9, payload_set(unauthorized_keys)),
            (10, text_set(diagnostic_codes)),
        ]))
        .map(Self)
    }
}

impl RegistrationFact {
    /// Constructs a complete syntax-registration fact.
    pub fn new(
        carrier: ObjectRef,
        parsed_item_digest: SemanticDigest,
        role: EvidenceRole,
        atoms: Vec<crate::model::AtomRevisionRef>,
        scope_digest: SemanticDigest,
        validity: Validity,
        diagnostic_codes: Vec<String>,
    ) -> Result<Self, EncodingError> {
        let atoms = CanonicalValue::Set(
            atoms
                .into_iter()
                .map(|atom| {
                    CanonicalValue::Record(alloc::vec![
                        (1, CanonicalValue::Text(atom.id().as_str().to_string())),
                        (2, CanonicalValue::Digest(atom.revision())),
                    ])
                })
                .collect(),
        );
        checked(CanonicalValue::Record(alloc::vec![
            (1, object_ref(&carrier)),
            (2, CanonicalValue::Digest(parsed_item_digest)),
            (3, enum_value(role)),
            (4, atoms),
            (5, CanonicalValue::Digest(scope_digest)),
            (6, enum_value(validity)),
            (7, text_set(diagnostic_codes)),
        ]))
        .map(Self)
    }
}

impl PolicyFact {
    /// Constructs a complete materialized policy fact.
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        policy: ObjectRef,
        subject: ObjectRef,
        obligation_value: ObligationRef,
        enforcement: EnforcementMode,
        applicability: Validity,
        satisfaction: Satisfaction,
        satisfying_references: Vec<ObjectRef>,
        active_debt: Vec<ObjectRef>,
        active_waivers: Vec<ObjectRef>,
        diagnostic_codes: Vec<String>,
    ) -> Result<Self, EncodingError> {
        checked(CanonicalValue::Record(alloc::vec![
            (1, object_ref(&policy)),
            (2, object_ref(&subject)),
            (3, obligation(&obligation_value)),
            (4, enum_value(enforcement)),
            (5, enum_value(applicability)),
            (6, enum_value(satisfaction)),
            (7, object_ref_set(satisfying_references)),
            (8, object_ref_set(active_debt)),
            (9, object_ref_set(active_waivers)),
            (10, text_set(diagnostic_codes)),
        ]))
        .map(Self)
    }
}

impl FreshnessFact {
    /// Constructs a complete binding-freshness fact.
    pub fn new(
        subject: ObjectRef,
        freshness: Freshness,
        expected_bindings: Vec<SemanticDigest>,
        current_bindings: Vec<SemanticDigest>,
        stale_binding_keys: Vec<String>,
        diagnostic_codes: Vec<String>,
    ) -> Result<Self, EncodingError> {
        checked(CanonicalValue::Record(alloc::vec![
            (1, object_ref(&subject)),
            (2, enum_value(freshness)),
            (3, digest_set(expected_bindings)),
            (4, digest_set(current_bindings)),
            (5, text_set(stale_binding_keys)),
            (6, text_set(diagnostic_codes)),
        ]))
        .map(Self)
    }
}

impl DebtWaiverFact {
    /// Constructs a complete debt/waiver lifecycle fact.
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        subject: ObjectRef,
        validity: Validity,
        lifecycle: GovernanceLifecycle,
        obligations: Vec<ObligationRef>,
        unsatisfied_obligations: Vec<ObligationRef>,
        owner: impl Into<String>,
        discharge_condition_digest: SemanticDigest,
        discharge_evidence: Vec<ObjectRef>,
        replacements: Vec<ObjectRef>,
        diagnostic_codes: Vec<String>,
    ) -> Result<Self, EncodingError> {
        checked(CanonicalValue::Record(alloc::vec![
            (1, object_ref(&subject)),
            (2, enum_value(validity)),
            (3, enum_value(lifecycle)),
            (4, obligation_set(obligations)),
            (5, obligation_set(unsatisfied_obligations)),
            (6, CanonicalValue::Text(owner.into())),
            (7, CanonicalValue::Digest(discharge_condition_digest)),
            (8, object_ref_set(discharge_evidence)),
            (9, object_ref_set(replacements)),
            (10, text_set(diagnostic_codes)),
        ]))
        .map(Self)
    }
}

impl DerivationFact {
    /// Constructs a complete derivation relation/freshness fact.
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        derivation: ObjectRef,
        freshness: Freshness,
        sources: Vec<ArtifactBinding>,
        targets: Vec<ArtifactBinding>,
        relation_tag: u16,
        consistency_oracle: ObjectRef,
        consistency: Consistency,
        scope_digest: SemanticDigest,
        exclusions: Vec<String>,
        diagnostic_codes: Vec<String>,
    ) -> Result<Self, EncodingError> {
        checked(CanonicalValue::Record(alloc::vec![
            (1, object_ref(&derivation)),
            (2, enum_value(freshness)),
            (3, artifact_set(sources)),
            (4, artifact_set(targets)),
            (5, enum_value(relation_tag)),
            (6, object_ref(&consistency_oracle)),
            (7, enum_value(consistency)),
            (8, CanonicalValue::Digest(scope_digest)),
            (9, text_set(exclusions)),
            (10, text_set(diagnostic_codes)),
        ]))
        .map(Self)
    }
}

impl ProofFact {
    /// Constructs a complete orthogonal proof fact.
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        proof: ObjectRef,
        registration_validity: Validity,
        correspondence_validity: Validity,
        binding_completeness: Completeness,
        freshness: Freshness,
        verifier_selection: Selection,
        verifier_verdict: Verdict,
        negative_fixture_satisfaction: Satisfaction,
        scope_digest: SemanticDigest,
        assumptions_exclusions: TypedPayload,
        formally_proved: Satisfaction,
        diagnostic_codes: Vec<String>,
    ) -> Result<Self, EncodingError> {
        checked(CanonicalValue::Record(alloc::vec![
            (1, object_ref(&proof)),
            (2, enum_value(registration_validity)),
            (3, enum_value(correspondence_validity)),
            (4, enum_value(binding_completeness)),
            (5, enum_value(freshness)),
            (6, enum_value(verifier_selection)),
            (7, enum_value(verifier_verdict)),
            (8, enum_value(negative_fixture_satisfaction)),
            (9, CanonicalValue::Digest(scope_digest)),
            (10, payload(&assumptions_exclusions)),
            (11, enum_value(formally_proved)),
            (12, text_set(diagnostic_codes)),
        ]))
        .map(Self)
    }
}

/// Derives one complete proof fact from all graph, artifact, tool,
/// correspondence, and negative-control invalidation dimensions.
///
/// # Errors
///
/// Returns canonical encoding errors from scope/fact construction.
///
/// # Panics
///
/// Panics only if a previously validated proof metadata or scope value can no
/// longer construct its corresponding canonical report value.
#[allow(clippy::expect_used)]
pub fn proof_report_fact(
    graph: &SpecGraph,
    proof: &ProofRecord,
    context: &ProofInvalidationContext<'_>,
    assumptions_exclusions: TypedPayload,
) -> Result<ProofFact, EncodingError> {
    let diagnostics = proof_invalidation_reasons(graph, proof, context);
    let contains = |code: &str| diagnostics.iter().any(|actual| actual == code);
    let registered = graph.proof_records.iter().any(|current| {
        (current.meta().id(), current.meta().digest()) == (proof.meta().id(), proof.meta().digest())
    });
    let fresh = diagnostics.is_empty();
    let correspondence_invalid =
        contains("FPR-BIND-CORRESPONDENCE") | contains("FPR-BIND-EXPECTATION");
    let negative_control_unsatisfied =
        contains("FPR-VERDICT-NEGATIVE-CONTROL") | contains("FPR-BIND-EXPECTATION");
    ProofFact::new(
        ObjectRef::new(
            ObjectKind::ProofRecord,
            proof.meta().id().clone(),
            proof.meta().digest(),
        ),
        if registered {
            Validity::Valid
        } else {
            Validity::Invalid
        },
        if correspondence_invalid {
            Validity::Invalid
        } else {
            Validity::Valid
        },
        if contains("FPR-BIND-EXPECTATION") {
            Completeness::Incomplete
        } else {
            Completeness::Complete
        },
        if fresh {
            Freshness::Fresh
        } else {
            Freshness::Stale
        },
        if contains("FPR-BIND-VERIFIER") {
            Selection::Invalid
        } else {
            Selection::Selected
        },
        proof.verdict(),
        if negative_control_unsatisfied {
            Satisfaction::Unsatisfied
        } else {
            Satisfaction::Satisfied
        },
        governance_payload_digest(proof.scope())
            .expect("validated proof scope constructs a canonical digest"),
        assumptions_exclusions,
        if (fresh, registered) == (true, true) {
            Satisfaction::Satisfied
        } else {
            Satisfaction::Unsatisfied
        },
        diagnostics,
    )
}

impl EvidenceFact {
    /// Constructs a complete execution evidence fact.
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        obligation_value: ObligationRef,
        carriers: Vec<ObjectRef>,
        registration_validity: Validity,
        freshness: Freshness,
        selection: Selection,
        verdicts: Vec<NormalizedVerdict>,
        claim_scope_digest: SemanticDigest,
        assumptions_exclusions: TypedPayload,
        assurance_classes: Vec<AssuranceClass>,
        satisfaction: Satisfaction,
        diagnostic_codes: Vec<String>,
    ) -> Result<Self, EncodingError> {
        checked(CanonicalValue::Record(alloc::vec![
            (1, obligation(&obligation_value)),
            (2, object_ref_set(carriers)),
            (3, enum_value(registration_validity)),
            (4, enum_value(freshness)),
            (5, enum_value(selection)),
            (
                6,
                CanonicalValue::Set(
                    verdicts
                        .iter()
                        .map(NormalizedVerdict::canonical_value)
                        .collect(),
                ),
            ),
            (7, CanonicalValue::Digest(claim_scope_digest)),
            (8, payload(&assumptions_exclusions)),
            (
                9,
                CanonicalValue::Set(assurance_classes.into_iter().map(enum_value).collect()),
            ),
            (10, enum_value(satisfaction)),
            (11, text_set(diagnostic_codes)),
        ]))
        .map(Self)
    }
}

impl DispositionClosureFact {
    /// Constructs a complete impact-disposition closure fact.
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        object_kind: ObjectKind,
        stable_id: StableId,
        base_digest: Option<SemanticDigest>,
        head_digest: Option<SemanticDigest>,
        distance: ImpactDistance,
        required_classes: Vec<ChangeDispositionKind>,
        matching_disposition: Option<ObjectRef>,
        closure: Closure,
        reaffirmation: Option<ObjectRef>,
        waiver: Option<ObjectRef>,
        diagnostic_codes: Vec<String>,
    ) -> Result<Self, EncodingError> {
        checked(CanonicalValue::Record(alloc::vec![
            (1, enum_value(object_kind)),
            (2, CanonicalValue::Text(stable_id.as_str().to_string())),
            (3, digest_option(base_digest)),
            (4, digest_option(head_digest)),
            (5, enum_value(distance)),
            (
                6,
                CanonicalValue::Set(required_classes.into_iter().map(enum_value).collect()),
            ),
            (7, object_ref_option(matching_disposition.as_ref())),
            (8, enum_value(closure)),
            (9, object_ref_option(reaffirmation.as_ref())),
            (10, object_ref_option(waiver.as_ref())),
            (11, text_set(diagnostic_codes)),
        ]))
        .map(Self)
    }
}

fn graph_has_reference(graph: &SpecGraph, reference: &ObjectRef) -> bool {
    graph.object_meta().into_iter().any(|meta| {
        (meta.kind(), meta.id(), meta.digest())
            == (reference.kind(), reference.id(), reference.digest())
    })
}

fn report_approval_current(approval: &ApprovalBinding, artifacts: &ArtifactIndex) -> bool {
    artifacts
        .entries()
        .iter()
        .any(|entry| (entry.path(), entry.raw_digest()) == (approval.record(), approval.digest()))
}

fn governance_payload_digest(payload: &TypedPayload) -> Result<SemanticDigest, EncodingError> {
    semantic_digest(
        "typed-payload",
        alloc::vec![
            (1, CanonicalValue::Text(payload.schema().to_string())),
            (2, CanonicalValue::Bytes(payload.bytes().to_vec())),
        ],
    )
}

/// Derives complete debt and waiver facts while preserving every obligation as
/// technically unsatisfied regardless of process authorization or lifecycle.
///
/// # Errors
///
/// Returns canonical encoding errors from fact construction or payload binding.
///
/// # Panics
///
/// Panics only if previously validated governance metadata, payloads, or facts
/// can no longer construct or encode their corresponding canonical values.
#[allow(clippy::expect_used, clippy::too_many_lines)]
pub fn governance_report_facts(
    graph: &SpecGraph,
    artifacts: &ArtifactIndex,
) -> Result<Vec<DebtWaiverFact>, EncodingError> {
    let mut facts = Vec::new();
    for debt in &graph.debt_records {
        let valid = (
            debt.obligations()
                .iter()
                .all(|obligation| graph_has_reference(graph, obligation.subject())),
            debt.discharge_evidence()
                .iter()
                .chain(debt.replacements())
                .all(|reference| graph_has_reference(graph, reference)),
            debt.terminal_approval()
                .is_none_or(|approval| report_approval_current(approval, artifacts)),
        ) == (true, true, true);
        let lifecycle = match debt.lifecycle() {
            DebtLifecycle::Active => GovernanceLifecycle::Active,
            DebtLifecycle::Discharged => GovernanceLifecycle::Discharged,
            DebtLifecycle::Superseded => GovernanceLifecycle::Superseded,
        };
        let diagnostics = if valid {
            Vec::new()
        } else {
            alloc::vec![String::from("EVD-LAUNDER-STALE-GOVERNANCE")]
        };
        facts.push(
            DebtWaiverFact::new(
                ObjectRef::new(
                    ObjectKind::DebtRecord,
                    debt.meta().id().clone(),
                    debt.meta().digest(),
                ),
                if valid {
                    Validity::Valid
                } else {
                    Validity::Invalid
                },
                lifecycle,
                debt.obligations().to_vec(),
                debt.obligations().to_vec(),
                debt.owner(),
                governance_payload_digest(debt.discharge_condition())
                    .expect("validated debt discharge conditions construct canonical digests"),
                debt.discharge_evidence().to_vec(),
                debt.replacements().to_vec(),
                diagnostics,
            )
            .expect("validated debt data constructs a canonical governance fact"),
        );
    }
    for waiver in &graph.waivers {
        let valid = (
            waiver
                .obligations()
                .iter()
                .all(|obligation| graph_has_reference(graph, obligation.subject())),
            waiver
                .discharge_evidence()
                .iter()
                .chain(waiver.replacements())
                .all(|reference| graph_has_reference(graph, reference)),
            report_approval_current(waiver.active_approval(), artifacts),
            waiver
                .terminal_approval()
                .is_none_or(|approval| report_approval_current(approval, artifacts)),
        ) == (true, true, true, true);
        let lifecycle = match waiver.lifecycle() {
            WaiverLifecycle::Active => GovernanceLifecycle::Active,
            WaiverLifecycle::Discharged => GovernanceLifecycle::Discharged,
            WaiverLifecycle::Revoked => GovernanceLifecycle::Revoked,
            WaiverLifecycle::Superseded => GovernanceLifecycle::Superseded,
        };
        let diagnostics = if valid {
            Vec::new()
        } else {
            alloc::vec![String::from("EVD-LAUNDER-STALE-GOVERNANCE")]
        };
        facts.push(
            DebtWaiverFact::new(
                ObjectRef::new(
                    ObjectKind::Waiver,
                    waiver.meta().id().clone(),
                    waiver.meta().digest(),
                ),
                if valid {
                    Validity::Valid
                } else {
                    Validity::Invalid
                },
                lifecycle,
                waiver.obligations().to_vec(),
                waiver.obligations().to_vec(),
                waiver.owner(),
                governance_payload_digest(waiver.discharge_condition())
                    .expect("validated waiver discharge conditions construct canonical digests"),
                waiver.discharge_evidence().to_vec(),
                waiver.replacements().to_vec(),
                diagnostics,
            )
            .expect("validated waiver data constructs a canonical governance fact"),
        );
    }
    let mut keyed = facts
        .into_iter()
        .map(|fact| {
            let bytes = fact
                .value()
                .encode()
                .expect("validated governance facts encode canonically");
            (bytes, fact)
        })
        .collect::<Vec<_>>();
    keyed.sort_unstable_by(|left, right| left.0.cmp(&right.0));
    Ok(keyed.into_iter().map(|(_, fact)| fact).collect())
}

fn required_disposition_classes(impact: &Impact) -> Vec<ChangeDispositionKind> {
    match (impact.base, impact.head) {
        (Some(base), Some(head)) if base != head => alloc::vec![
            ChangeDispositionKind::Reaffirmed,
            ChangeDispositionKind::Revised,
            ChangeDispositionKind::Waived,
        ],
        (Some(_), None) => alloc::vec![
            ChangeDispositionKind::Retired,
            ChangeDispositionKind::Waived,
        ],
        (None, Some(_)) => alloc::vec![ChangeDispositionKind::Waived],
        _ => Vec::new(),
    }
}

/// Derives orthogonal closure facts without allowing reaffirmations or waivers
/// to alter impact identity, technical freshness, or satisfaction.
///
/// # Errors
///
/// Returns canonical encoding failures from exact fact/reference construction.
///
/// # Panics
///
/// Panics only if previously validated disposition, reaffirmation, or waiver
/// metadata can no longer construct its corresponding canonical reference.
#[allow(clippy::expect_used, clippy::too_many_lines)]
pub fn disposition_report_facts(
    impacts: &[Impact],
    manifest: &ChangeManifest,
    context: &DispositionProtocolContext<'_>,
) -> Result<Vec<DispositionClosureFact>, EncodingError> {
    let diagnostics = disposition_protocol_closure(impacts, manifest, context);
    let mut facts = Vec::new();
    for impact in impacts {
        let matching = manifest
            .dispositions()
            .iter()
            .filter(|disposition| {
                (disposition.impacted_kind(), disposition.impacted_id())
                    == (impact.kind, &impact.id)
            })
            .collect::<Vec<_>>();
        let codes = diagnostics
            .iter()
            .filter(|diagnostic| {
                (diagnostic.kind, diagnostic.id.as_deref())
                    == (Some(impact.kind), Some(impact.id.as_str()))
            })
            .map(|diagnostic| diagnostic.code.clone())
            .collect::<Vec<_>>();
        let matching_reference = matching.first().map(|disposition| {
            ObjectRef::new(
                ObjectKind::ChangeDisposition,
                disposition.meta().id().clone(),
                disposition.meta().digest(),
            )
        });
        let reaffirmation = matching
            .first()
            .and_then(|disposition| {
                let digest = disposition.reaffirmation_digest()?;
                context.head().reaffirmations.iter().find(|record| {
                    (
                        record.meta().digest(),
                        record.affected_object_kind(),
                        record.affected_object_id(),
                        Some(record.base_object_digest()),
                        Some(record.head_object_digest()),
                    ) == (digest, impact.kind, &impact.id, impact.base, impact.head)
                })
            })
            .map(|record| {
                ObjectRef::new(
                    ObjectKind::Reaffirmation,
                    record.meta().id().clone(),
                    record.meta().digest(),
                )
            });
        let waiver = matching
            .first()
            .and_then(|disposition| disposition.waiver())
            .and_then(|reference| {
                context.head().waivers.iter().find(|waiver| {
                    (
                        waiver.lifecycle(),
                        waiver.meta().id(),
                        waiver.meta().digest(),
                    ) == (WaiverLifecycle::Active, reference.id(), reference.digest())
                })
            })
            .map(|waiver| {
                ObjectRef::new(
                    ObjectKind::Waiver,
                    waiver.meta().id().clone(),
                    waiver.meta().digest(),
                )
            });
        let closure = if matching.len() != 1 {
            Closure::Open
        } else if codes.is_empty() {
            Closure::Closed
        } else {
            Closure::Invalid
        };
        facts.push(
            DispositionClosureFact::new(
                impact.kind,
                impact.id.clone(),
                impact.base,
                impact.head,
                if impact.direct {
                    ImpactDistance::Direct
                } else {
                    ImpactDistance::Transitive
                },
                required_disposition_classes(impact),
                matching_reference,
                closure,
                reaffirmation,
                waiver,
                codes,
            )
            .expect("validated impact data constructs a canonical disposition fact"),
        );
    }
    let mut keyed = facts
        .into_iter()
        .map(|fact| {
            let bytes = fact
                .value()
                .encode()
                .expect("validated disposition facts encode canonically");
            (bytes, fact)
        })
        .collect::<Vec<_>>();
    keyed.sort_unstable_by(|left, right| left.0.cmp(&right.0));
    Ok(keyed.into_iter().map(|(_, fact)| fact).collect())
}

impl CanonicalDiagnostic {
    /// Converts the stable canonical portion of a core diagnostic.
    pub fn new(
        diagnostic: &Diagnostic,
        severity: DiagnosticSeverity,
    ) -> Result<Self, EncodingError> {
        let location = diagnostic
            .path
            .as_ref()
            .map_or(CanonicalValue::None, |path| {
                CanonicalValue::Some(Box::new(CanonicalValue::Record(alloc::vec![
                    (1, CanonicalValue::Text(path.as_str().to_string())),
                    (2, CanonicalValue::U64(diagnostic.start)),
                    (3, CanonicalValue::U64(diagnostic.end)),
                ])))
            });
        let kind = diagnostic.kind.map_or(CanonicalValue::None, |kind| {
            CanonicalValue::Some(Box::new(enum_value(kind)))
        });
        let id = diagnostic.id.as_ref().map_or(CanonicalValue::None, |id| {
            CanonicalValue::Some(Box::new(CanonicalValue::Text(id.clone())))
        });
        let fields = CanonicalValue::Set(
            diagnostic
                .fields
                .iter()
                .map(|(key, value)| {
                    CanonicalValue::Record(alloc::vec![
                        (1, CanonicalValue::Text(key.clone())),
                        (2, CanonicalValue::Text(value.clone())),
                    ])
                })
                .collect(),
        );
        checked(CanonicalValue::Record(alloc::vec![
            (1, CanonicalValue::Text(diagnostic.code.clone())),
            (2, location),
            (3, kind),
            (4, id),
            (5, CanonicalValue::Record(alloc::vec![(1, fields)])),
            (6, enum_value(severity)),
            (7, CanonicalValue::None),
        ]))
        .map(Self)
    }
}

/// Strict report decoding failure.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ReportDecodingError {
    /// Generic canonical bytes are malformed or noncanonical.
    Canonical(DecodingError),
    /// Canonical value does not have the exact report field table and shapes.
    Schema,
    /// Canonical report bytes could not be re-bound to a domain digest.
    Identity,
}

impl ReportDecodingError {
    /// Stable diagnostic code.
    #[must_use]
    pub const fn code(self) -> &'static str {
        match self {
            Self::Canonical(error) => error.code(),
            Self::Schema => "DET-ENC-REPORT-SCHEMA",
            Self::Identity => "DET-ENC-REPORT-IDENTITY",
        }
    }
}

fn report_fields(
    value: &CanonicalValue,
    expected: usize,
    set_fields: &[usize],
) -> Result<Vec<(u16, CanonicalValue)>, ReportDecodingError> {
    let CanonicalValue::Record(fields) = value else {
        return Err(ReportDecodingError::Schema);
    };
    if fields.len() != expected {
        return Err(ReportDecodingError::Schema);
    }
    for (index, (tag, value)) in fields.iter().enumerate() {
        if usize::from(*tag) != index + 1 {
            return Err(ReportDecodingError::Schema);
        }
        let expected_set = set_fields.contains(&(index + 1));
        if expected_set != matches!(value, CanonicalValue::Set(_)) {
            return Err(ReportDecodingError::Schema);
        }
        if !expected_set && !matches!(value, CanonicalValue::Digest(_)) {
            return Err(ReportDecodingError::Schema);
        }
    }
    Ok(fields.clone())
}

/// Authoritative canonical static report.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CanonicalStaticReport {
    value: CanonicalValue,
    bytes: Vec<u8>,
    digest: SemanticDigest,
}

impl CanonicalStaticReport {
    /// Strictly decodes one exact complete static-report binary value.
    ///
    /// # Errors
    ///
    /// Rejects malformed canonical bytes and unknown, omitted, duplicate,
    /// out-of-order, or wrong-shaped report fields.
    pub fn decode(bytes: &[u8], budget: DecodeBudget) -> Result<Self, ReportDecodingError> {
        let value =
            CanonicalValue::decode(bytes, budget).map_err(ReportDecodingError::Canonical)?;
        report_fields(&value, 10, &[2, 3, 4, 5, 6, 7, 8, 9, 10])?;
        let digest = semantic_digest_from_encoded_record("static-report", bytes);
        Ok(Self {
            value,
            bytes: bytes.to_vec(),
            digest,
        })
    }

    /// Constructs the complete static report with every required fact set.
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        complete_graph_digest: SemanticDigest,
        authority_facts: Vec<AuthorityFact>,
        surface_facts: Vec<SurfaceFact>,
        registration_facts: Vec<RegistrationFact>,
        policy_facts: Vec<PolicyFact>,
        freshness_facts: Vec<FreshnessFact>,
        debt_waiver_facts: Vec<DebtWaiverFact>,
        derivation_facts: Vec<DerivationFact>,
        proof_facts: Vec<ProofFact>,
        diagnostics: Vec<CanonicalDiagnostic>,
    ) -> Result<Self, EncodingError> {
        let fields = alloc::vec![
            (1, CanonicalValue::Digest(complete_graph_digest)),
            (
                2,
                CanonicalValue::Set(authority_facts.iter().map(AuthorityFact::value).collect())
            ),
            (
                3,
                CanonicalValue::Set(surface_facts.iter().map(SurfaceFact::value).collect())
            ),
            (
                4,
                CanonicalValue::Set(
                    registration_facts
                        .iter()
                        .map(RegistrationFact::value)
                        .collect()
                )
            ),
            (
                5,
                CanonicalValue::Set(policy_facts.iter().map(PolicyFact::value).collect())
            ),
            (
                6,
                CanonicalValue::Set(freshness_facts.iter().map(FreshnessFact::value).collect())
            ),
            (
                7,
                CanonicalValue::Set(
                    debt_waiver_facts
                        .iter()
                        .map(DebtWaiverFact::value)
                        .collect()
                )
            ),
            (
                8,
                CanonicalValue::Set(derivation_facts.iter().map(DerivationFact::value).collect())
            ),
            (
                9,
                CanonicalValue::Set(proof_facts.iter().map(ProofFact::value).collect())
            ),
            (
                10,
                CanonicalValue::Set(diagnostics.iter().map(CanonicalDiagnostic::value).collect())
            ),
        ];
        let value = CanonicalValue::Record(fields);
        let bytes = value.encode()?;
        let digest = semantic_digest_from_encoded_record("static-report", &bytes);
        Ok(Self {
            value,
            bytes,
            digest,
        })
    }

    /// Exact authoritative binary bytes.
    #[must_use]
    pub fn bytes(&self) -> &[u8] {
        &self.bytes
    }

    /// Domain-separated authoritative report identity.
    #[must_use]
    pub const fn digest(&self) -> SemanticDigest {
        self.digest
    }

    /// Complete canonical value for independent presentation derivation.
    #[must_use]
    pub const fn value(&self) -> &CanonicalValue {
        &self.value
    }
}

/// Authoritative canonical execution report.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CanonicalExecutionReport {
    value: CanonicalValue,
    bytes: Vec<u8>,
    digest: SemanticDigest,
}

impl CanonicalExecutionReport {
    /// Strictly decodes one exact complete execution-report binary value.
    ///
    /// # Errors
    ///
    /// Rejects malformed canonical bytes and unknown, omitted, duplicate,
    /// out-of-order, or wrong-shaped report fields.
    pub fn decode(bytes: &[u8], budget: DecodeBudget) -> Result<Self, ReportDecodingError> {
        let value =
            CanonicalValue::decode(bytes, budget).map_err(ReportDecodingError::Canonical)?;
        report_fields(&value, 4, &[3, 4])?;
        let digest = semantic_digest_from_encoded_record("execution-report", bytes);
        Ok(Self {
            value,
            bytes: bytes.to_vec(),
            digest,
        })
    }

    /// Constructs a complete execution report with explicit empty sets retained.
    pub fn new(
        static_graph_digest: SemanticDigest,
        verdicts: Vec<NormalizedVerdict>,
        evidence_facts: Vec<EvidenceFact>,
        diagnostics: Vec<CanonicalDiagnostic>,
    ) -> Result<Self, EncodingError> {
        let verdict_values = CanonicalValue::Set(
            verdicts
                .iter()
                .map(NormalizedVerdict::canonical_value)
                .collect(),
        );
        let verdict_set_digest = semantic_digest("verdict-set", alloc::vec![(1, verdict_values)])?;
        let fields = alloc::vec![
            (1, CanonicalValue::Digest(static_graph_digest)),
            (2, CanonicalValue::Digest(verdict_set_digest)),
            (
                3,
                CanonicalValue::Set(evidence_facts.iter().map(EvidenceFact::value).collect())
            ),
            (
                4,
                CanonicalValue::Set(diagnostics.iter().map(CanonicalDiagnostic::value).collect())
            ),
        ];
        let value = CanonicalValue::Record(fields);
        let bytes = value.encode()?;
        let digest = semantic_digest_from_encoded_record("execution-report", &bytes);
        Ok(Self {
            value,
            bytes,
            digest,
        })
    }

    /// Exact authoritative binary bytes.
    #[must_use]
    pub fn bytes(&self) -> &[u8] {
        &self.bytes
    }

    /// Domain-separated authoritative report identity.
    #[must_use]
    pub const fn digest(&self) -> SemanticDigest {
        self.digest
    }

    /// Complete canonical value for independent presentation derivation.
    #[must_use]
    pub const fn value(&self) -> &CanonicalValue {
        &self.value
    }
}

/// Authoritative canonical impact report.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CanonicalImpactReport {
    value: CanonicalValue,
    bytes: Vec<u8>,
    digest: SemanticDigest,
}

impl CanonicalImpactReport {
    /// Strictly decodes one exact complete impact-report binary value.
    ///
    /// # Errors
    ///
    /// Rejects malformed canonical bytes and unknown, omitted, duplicate,
    /// out-of-order, or wrong-shaped report fields.
    pub fn decode(bytes: &[u8], budget: DecodeBudget) -> Result<Self, ReportDecodingError> {
        let value =
            CanonicalValue::decode(bytes, budget).map_err(ReportDecodingError::Canonical)?;
        report_fields(&value, 6, &[5, 6])?;
        let digest = semantic_digest_from_encoded_record("impact-report", bytes);
        Ok(Self {
            value,
            bytes: bytes.to_vec(),
            digest,
        })
    }

    /// Constructs a complete impact report with explicit closure facts.
    pub fn new(
        base_graph: SemanticDigest,
        head_graph: SemanticDigest,
        change_manifest: SemanticDigest,
        impact_set: SemanticDigest,
        closure_facts: Vec<DispositionClosureFact>,
        diagnostics: Vec<CanonicalDiagnostic>,
    ) -> Result<Self, EncodingError> {
        let fields = alloc::vec![
            (1, CanonicalValue::Digest(base_graph)),
            (2, CanonicalValue::Digest(head_graph)),
            (3, CanonicalValue::Digest(change_manifest)),
            (4, CanonicalValue::Digest(impact_set)),
            (
                5,
                CanonicalValue::Set(
                    closure_facts
                        .iter()
                        .map(DispositionClosureFact::value)
                        .collect()
                )
            ),
            (
                6,
                CanonicalValue::Set(diagnostics.iter().map(CanonicalDiagnostic::value).collect())
            ),
        ];
        let value = CanonicalValue::Record(fields);
        let bytes = value.encode()?;
        let digest = semantic_digest_from_encoded_record("impact-report", &bytes);
        Ok(Self {
            value,
            bytes,
            digest,
        })
    }

    /// Exact authoritative binary bytes.
    #[must_use]
    pub fn bytes(&self) -> &[u8] {
        &self.bytes
    }

    /// Domain-separated authoritative report identity.
    #[must_use]
    pub const fn digest(&self) -> SemanticDigest {
        self.digest
    }

    /// Complete canonical value for independent presentation derivation.
    #[must_use]
    pub const fn value(&self) -> &CanonicalValue {
        &self.value
    }
}
