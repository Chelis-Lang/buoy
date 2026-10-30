//! Strict closed adapter schemas translated into complete `buoy-core` values.

use super::MarkdownSemantic;
use buoy_core::{
    ApprovalBinding, ArtifactBinding, Assumption, Atom, AtomKind, AtomRevisionRef,
    AuthorityExpectation, CoveragePolicy, CoveragePolicyRevision, DebtLifecycle, DebtRecord,
    DecisionSurface, DecisionSurfaceDomain, Derivation, DerivationRelation, EnforcementMode,
    EvidenceCarrier, EvidenceRole, FiniteModel, FormalProperty, ImplementationLink, LifecycleState,
    NavigationKind, ObjectKind, ObjectRef, ObligationKind, ObligationRef, PolicyLifecycle,
    ProofRecord, Reaffirmation, RepositoryPath, SchemaId, ScopeId, SemanticDigest, StableId,
    SurfaceDisposition, SurfaceDispositionKind, ToolIdentity, TypedPayload, TypedSelector, Verdict,
    Waiver, WaiverLifecycle,
};
use serde::Deserialize;
use serde::de::DeserializeOwned;
use unicode_normalization::UnicodeNormalization;

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct ParseIssue {
    pub(crate) code: &'static str,
    pub(crate) context: String,
}

fn parse_scope_id(value: &str) -> Result<ScopeId, ParseIssue> {
    ScopeId::new(value).map_err(|error| ParseIssue::malformed(error.code()))
}

fn parse_schema_id(value: &str) -> Result<SchemaId, ParseIssue> {
    SchemaId::new(value).map_err(|error| ParseIssue::malformed(error.code()))
}

impl ParseIssue {
    fn malformed(context: impl Into<String>) -> Self {
        Self {
            code: "SPG-ADAPTER-MALFORMED-OBJECT",
            context: context.into(),
        }
    }

    fn malformed_atom(context: impl Into<String>) -> Self {
        Self {
            code: "SPG-ADAPTER-MALFORMED-ATOM",
            context: context.into(),
        }
    }

    fn rust(context: impl Into<String>) -> Self {
        Self {
            code: "SPG-ADAPTER-MALFORMED-ATTRIBUTE",
            context: context.into(),
        }
    }
}

pub(crate) fn parse_markdown_semantic(
    syntax: &str,
    body: &str,
) -> Result<MarkdownSemantic, ParseIssue> {
    match syntax {
        "spec-provenance-atom-v1" => parse_atom(body).map(MarkdownSemantic::Atom),
        "spec-provenance-surface-v1" => parse_surface(body).map(MarkdownSemantic::DecisionSurface),
        "spec-provenance-policy-v1" => parse_policy(body).map(MarkdownSemantic::CoveragePolicy),
        "spec-provenance-debt-v1" => parse_debt(body).map(MarkdownSemantic::DebtRecord),
        "spec-provenance-waiver-v1" => parse_waiver(body).map(MarkdownSemantic::Waiver),
        "spec-provenance-reaffirmation-v1" => {
            parse_reaffirmation(body).map(MarkdownSemantic::Reaffirmation)
        }
        "spec-provenance-derivation-v1" => parse_derivation(body).map(MarkdownSemantic::Derivation),
        "spec-provenance-assumption-v1" => parse_assumption(body).map(MarkdownSemantic::Assumption),
        "spec-provenance-finite-model-v1" => {
            parse_finite_model(body).map(MarkdownSemantic::FiniteModel)
        }
        "spec-provenance-formal-property-v1" => {
            parse_formal_property(body).map(MarkdownSemantic::FormalProperty)
        }
        "spec-provenance-proof-record-v1" => {
            parse_proof_record(body).map(MarkdownSemantic::ProofRecord)
        }
        _ => Err(ParseIssue {
            code: "SPG-ADAPTER-UNSUPPORTED-SYNTAX-VERSION",
            context: syntax.to_owned(),
        }),
    }
}

pub(crate) fn parse_surface(body: &str) -> Result<DecisionSurface, ParseIssue> {
    let value: SurfaceDef = decode(body)?;
    let domain = DecisionSurfaceDomain::new(
        StableId::new(&value.id).map_err(id_issue)?,
        value.member_key_schema.build()?,
        value.disposition_vocabulary.build()?,
        build_all(value.member_keys, PayloadDef::build)?,
        value.generation,
    )
    .map_err(core_issue)?;
    let dispositions = build_all(value.dispositions, DispositionDef::build)?;
    if domain.member_keys().len() != dispositions.len()
        || domain
            .member_keys()
            .iter()
            .any(|member| !dispositions.iter().any(|row| row.key() == member))
    {
        return Err(ParseIssue::malformed("surface-completeness"));
    }
    DecisionSurface::new(
        domain,
        dispositions,
        build_all(value.policies, ObjectRefDef::build)?,
    )
    .map_err(core_issue)
}

pub(crate) fn parse_derivation(body: &str) -> Result<Derivation, ParseIssue> {
    let value: DerivationDef = decode(body)?;
    Derivation::new(
        StableId::new(&value.id).map_err(id_issue)?,
        build_all(value.sources, ArtifactDef::build)?,
        build_all(value.targets, ArtifactDef::build)?,
        value.tool.build()?,
        digest(&value.configuration)?,
        value.oracle.build()?,
        derivation_relation(&value.relation)?,
        value.relation_scope.build()?,
        value.exclusions,
    )
    .map_err(core_issue)
}

pub(crate) fn parse_carrier(
    body: &str,
    parsed_item_digest: SemanticDigest,
    required_role: Option<EvidenceRole>,
) -> Result<EvidenceCarrier, ParseIssue> {
    let value: CarrierDef = decode_rust(body)?;
    (|| {
        let role = evidence_role(&value.role).map_err(|_| ParseIssue {
            code: "SPG-ADAPTER-CARRIER-ROLE",
            context: String::from("role"),
        })?;
        if required_role.is_some_and(|required| required != role) {
            return Err(ParseIssue {
                code: "SPG-ADAPTER-CARRIER-ROLE",
                context: String::from("attachment-role"),
            });
        }
        EvidenceCarrier::new(
            StableId::new(&value.id).map_err(|error| ParseIssue {
                code: error.code(),
                context: String::from("carrier"),
            })?,
            role,
            build_all(value.atoms, AtomRefDef::build).map_err(|_| ParseIssue::rust("atoms"))?,
            parsed_item_digest,
            value
                .oracle
                .build()
                .map_err(|_| ParseIssue::rust("oracle"))?,
            digest(&value.configuration).map_err(|_| ParseIssue::rust("configuration"))?,
            value
                .corpora
                .iter()
                .map(|value| digest(value))
                .collect::<Result<Vec<_>, _>>()
                .map_err(|_| ParseIssue::rust("corpora"))?,
            value.scope.build().map_err(|_| ParseIssue::rust("scope"))?,
            build_all(value.assumptions, ObjectRefDef::build)
                .map_err(|_| ParseIssue::rust("assumptions"))?,
            value.exclusions,
        )
        .map_err(|error| ParseIssue {
            code: error.code(),
            context: String::from("carrier"),
        })
    })()
}

pub(crate) fn parse_implementation_link(
    body: &str,
    parsed_item_digest: SemanticDigest,
) -> Result<ImplementationLink, ParseIssue> {
    let value: ImplementationDef = decode_rust(body)?;
    ImplementationLink::new(
        StableId::new(&value.id).map_err(|error| ParseIssue {
            code: error.code(),
            context: String::from("implementation-link"),
        })?,
        build_all(value.atoms, AtomRefDef::build)?,
        parsed_item_digest,
        navigation_kind(&value.navigation)?,
    )
    .map_err(|error| ParseIssue {
        code: error.code(),
        context: String::from("implementation-link"),
    })
}

fn parse_atom(body: &str) -> Result<Atom, ParseIssue> {
    let value: AtomDef = decode_atom(body)?;
    (|| {
        let state = match value.state.as_str() {
            "proposed" => LifecycleState::Proposed,
            "ratified" => LifecycleState::Ratified,
            _ => return Err(ParseIssue::malformed_atom("state")),
        };
        let kind = match value.kind.as_str() {
            "behavioral" => AtomKind::Behavioral,
            "structural" => AtomKind::Structural,
            "constraint" => AtomKind::Constraint,
            _ => return Err(ParseIssue::malformed_atom("kind")),
        };
        let statement = normalize_statement(&value.statement);
        Atom::new(
            StableId::new(&value.id).map_err(id_issue)?,
            state,
            kind,
            value
                .scope
                .into_iter()
                .map(|scope| parse_scope_id(&scope))
                .collect::<Result<Vec<_>, _>>()?,
            statement,
            value.approval.map(ApprovalDef::build).transpose()?,
        )
        .map_err(core_issue)
    })()
    .map_err(atom_issue)
}

fn parse_policy(body: &str) -> Result<CoveragePolicy, ParseIssue> {
    let value: PolicyDef = decode(body)?;
    let revision = CoveragePolicyRevision::new(
        value.scope.build()?,
        value
            .required_roles
            .iter()
            .map(|role| evidence_role(role))
            .collect::<Result<Vec<_>, _>>()?,
        enforcement_mode(&value.enforcement)?,
        parse_scope_id(&value.ratchet_id)?,
        build_all(value.authority_selectors, SelectorDef::build)?,
        value
            .derivation_requirements
            .map(PayloadDef::build)
            .transpose()?,
    )
    .map_err(core_issue)?;
    CoveragePolicy::new(
        StableId::new(&value.id).map_err(id_issue)?,
        revision,
        value.active_approval.build()?,
        policy_lifecycle(&value.lifecycle)?,
        build_all(value.replacements, ObjectRefDef::build)?,
        value
            .terminal_approval
            .map(ApprovalDef::build)
            .transpose()?,
        Vec::new(),
    )
    .map_err(core_issue)
}

fn parse_debt(body: &str) -> Result<DebtRecord, ParseIssue> {
    let value: DebtDef = decode(body)?;
    DebtRecord::new(
        StableId::new(&value.id).map_err(id_issue)?,
        build_all(value.obligations, ObligationDef::build)?,
        build_all(value.objects, ObjectRefDef::build)?,
        value.rationale,
        parse_scope_id(&value.owner)?,
        value.remediation,
        value.discharge_condition.build()?,
        value.external_references,
        debt_lifecycle(&value.lifecycle)?,
        build_all(value.discharge_evidence, ObjectRefDef::build)?,
        build_all(value.replacements, ObjectRefDef::build)?,
        value
            .terminal_approval
            .map(ApprovalDef::build)
            .transpose()?,
    )
    .map_err(core_issue)
}

fn parse_waiver(body: &str) -> Result<Waiver, ParseIssue> {
    let value: WaiverDef = decode(body)?;
    Waiver::new(
        StableId::new(&value.id).map_err(id_issue)?,
        build_all(value.obligations, ObligationDef::build)?,
        build_all(value.objects, ObjectRefDef::build)?,
        value.rationale,
        parse_scope_id(&value.owner)?,
        value.discharge_condition.build()?,
        value.active_approval.build()?,
        value.external_references,
        waiver_lifecycle(&value.lifecycle)?,
        build_all(value.discharge_evidence, ObjectRefDef::build)?,
        build_all(value.replacements, ObjectRefDef::build)?,
        value
            .terminal_approval
            .map(ApprovalDef::build)
            .transpose()?,
    )
    .map_err(core_issue)
}

fn parse_reaffirmation(body: &str) -> Result<Reaffirmation, ParseIssue> {
    let value: ReaffirmationDef = decode(body)?;
    Reaffirmation::new(
        StableId::new(&value.id).map_err(id_issue)?,
        object_kind(&value.affected_object_kind)?,
        StableId::new(&value.affected_object_id).map_err(id_issue)?,
        digest(&value.base_object_digest)?,
        digest(&value.head_object_digest)?,
        value.review_approval.build()?,
        value.review_statement,
    )
    .map_err(core_issue)
}

fn parse_assumption(body: &str) -> Result<Assumption, ParseIssue> {
    let value: AssumptionDef = decode(body)?;
    Assumption::new(
        StableId::new(&value.id).map_err(id_issue)?,
        value.statement,
        value.scope.build()?,
        value.exclusions,
        value.approval.build()?,
    )
    .map_err(core_issue)
}

fn parse_finite_model(body: &str) -> Result<FiniteModel, ParseIssue> {
    let value: FiniteModelDef = decode(body)?;
    FiniteModel::new(
        StableId::new(&value.id).map_err(id_issue)?,
        value.state_space.build()?,
        value.event_alphabet.build()?,
        value.transition_relation.build()?,
        build_all(value.initial_states, PayloadDef::build)?,
        value.bound.build()?,
        build_all(value.checked_invariants, AtomRefDef::build)?,
        build_all(value.assumptions, ObjectRefDef::build)?,
        value.exclusions,
    )
    .map_err(core_issue)
}

fn parse_formal_property(body: &str) -> Result<FormalProperty, ParseIssue> {
    let value: FormalPropertyDef = decode(body)?;
    FormalProperty::new(
        StableId::new(&value.id).map_err(id_issue)?,
        build_all(value.atoms, AtomRefDef::build)?,
        value.property.build()?,
        value.implementation_abstraction.build()?,
        build_all(value.assumptions, ObjectRefDef::build)?,
        value.exclusions,
        value.correspondence_approval.build()?,
    )
    .map_err(core_issue)
}

fn parse_proof_record(body: &str) -> Result<ProofRecord, ParseIssue> {
    let value: ProofRecordDef = decode(body)?;
    ProofRecord::new(
        StableId::new(&value.id).map_err(id_issue)?,
        build_all(value.atoms, AtomRefDef::build)?,
        value.property.build()?,
        value.implementation.build()?,
        build_all(value.assumptions, ObjectRefDef::build)?,
        build_all(value.translators, ToolDef::build)?,
        value.verifier.build()?,
        digest(&value.verifier_configuration)?,
        build_all(value.external_models, ArtifactDef::build)?,
        build_all(value.generated, ArtifactDef::build)?,
        build_all(value.proof_sources, ArtifactDef::build)?,
        verdict(&value.verdict)?,
        value.scope.build()?,
        value.exclusions,
    )
    .map_err(core_issue)
}

fn decode<T: DeserializeOwned>(body: &str) -> Result<T, ParseIssue> {
    toml::from_str(body).map_err(|error| classify_toml_error(&error))
}

fn decode_atom<T: DeserializeOwned>(body: &str) -> Result<T, ParseIssue> {
    toml::from_str(body).map_err(|error| {
        let mut issue = classify_toml_error(&error);
        if issue.code == "SPG-ADAPTER-MALFORMED-OBJECT" {
            issue.code = "SPG-ADAPTER-MALFORMED-ATOM";
        }
        issue
    })
}

fn decode_rust<T: DeserializeOwned>(body: &str) -> Result<T, ParseIssue> {
    toml::from_str(body).map_err(|error| {
        let classified = classify_toml_error(&error);
        let code = if matches!(
            classified.code,
            "SPG-ADAPTER-UNSUPPORTED-FIELD"
                | "SPG-ADAPTER-MISSING-FIELD"
                | "SPG-ADAPTER-DUPLICATE-FIELD"
        ) {
            classified.code
        } else {
            "SPG-ADAPTER-MALFORMED-ATTRIBUTE"
        };
        ParseIssue {
            code,
            context: classified.context,
        }
    })
}

fn classify_toml_error(error: &toml::de::Error) -> ParseIssue {
    let message = error.to_string();
    if message.contains("unknown field") {
        ParseIssue {
            code: "SPG-ADAPTER-UNSUPPORTED-FIELD",
            context: String::from("unknown-field"),
        }
    } else if message.contains("missing field") {
        ParseIssue {
            code: "SPG-ADAPTER-MISSING-FIELD",
            context: String::from("missing-field"),
        }
    } else if matches_duplicate_field_wording(&message) {
        ParseIssue {
            code: "SPG-ADAPTER-DUPLICATE-FIELD",
            context: String::from("duplicate-field"),
        }
    } else {
        ParseIssue::malformed("toml")
    }
}

/// Uses eager `|` rather than short-circuiting `||` so this reduces to a
/// single decision point for coverage purposes: the pinned `toml` release
/// only ever emits "duplicate key", so a short-circuited second operand for
/// the "duplicate field" synonym could never be exercised by real parsing.
fn matches_duplicate_field_wording(message: &str) -> bool {
    message.contains("duplicate key") | message.contains("duplicate field")
}

fn core_issue(error: buoy_core::ConstructionError) -> ParseIssue {
    ParseIssue::malformed(error.code())
}

fn id_issue(error: buoy_core::IdentifierError) -> ParseIssue {
    ParseIssue::malformed(error.code())
}

fn atom_issue(issue: ParseIssue) -> ParseIssue {
    if matches!(
        issue.code,
        "SPG-ADAPTER-UNSUPPORTED-FIELD"
            | "SPG-ADAPTER-MISSING-FIELD"
            | "SPG-ADAPTER-DUPLICATE-FIELD"
    ) {
        issue
    } else {
        ParseIssue::malformed_atom(issue.context)
    }
}

fn build_all<T, U>(
    values: Vec<T>,
    build: fn(T) -> Result<U, ParseIssue>,
) -> Result<Vec<U>, ParseIssue> {
    values.into_iter().map(build).collect()
}

fn digest(value: &str) -> Result<SemanticDigest, ParseIssue> {
    SemanticDigest::parse_canonical(value).map_err(|error| ParseIssue::malformed(error.code()))
}

fn decode_hex(value: &str) -> Result<Vec<u8>, ParseIssue> {
    if value.len() % 2 != 0
        || value
            .bytes()
            .any(|byte| !byte.is_ascii_digit() && !(b'a'..=b'f').contains(&byte))
    {
        return Err(ParseIssue::malformed("hex"));
    }
    Ok(value
        .as_bytes()
        .chunks_exact(2)
        .map(|pair| (hex_nibble(pair[0]) << 4) | hex_nibble(pair[1]))
        .collect())
}

/// Callers must guarantee `value` is an ASCII digit or `a`-`f`; `decode_hex`'s
/// guard filters every byte before any call reaches here.
fn hex_nibble(value: u8) -> u8 {
    match value {
        b'0'..=b'9' => value - b'0',
        _ => value - b'a' + 10,
    }
}

fn normalize_statement(input: &str) -> String {
    let normalized = input.replace("\r\n", "\n");
    let lines = normalized
        .lines()
        .map(|line| line.trim_end_matches([' ', '\t']))
        .collect::<Vec<_>>();
    let first = lines.iter().position(|line| !line.is_empty()).unwrap_or(0);
    let last = lines
        .iter()
        .rposition(|line| !line.is_empty())
        .map_or(first, |index| index + 1);
    lines[first..last].join("\n").nfc().collect()
}

fn object_kind(value: &str) -> Result<ObjectKind, ParseIssue> {
    match value {
        "atom" => Ok(ObjectKind::Atom),
        "tombstone" => Ok(ObjectKind::Tombstone),
        "decision-surface" => Ok(ObjectKind::DecisionSurface),
        "evidence-carrier" => Ok(ObjectKind::EvidenceCarrier),
        "implementation-link" => Ok(ObjectKind::ImplementationLink),
        "derivation" => Ok(ObjectKind::Derivation),
        "coverage-policy" => Ok(ObjectKind::CoveragePolicy),
        "debt-record" => Ok(ObjectKind::DebtRecord),
        "waiver" => Ok(ObjectKind::Waiver),
        "oracle" => Ok(ObjectKind::Oracle),
        "finite-model" => Ok(ObjectKind::FiniteModel),
        "formal-property" => Ok(ObjectKind::FormalProperty),
        "assumption" => Ok(ObjectKind::Assumption),
        "proof-record" => Ok(ObjectKind::ProofRecord),
        "parsed-item" => Ok(ObjectKind::ParsedItem),
        "reaffirmation" => Ok(ObjectKind::Reaffirmation),
        "change-manifest" => Ok(ObjectKind::ChangeManifest),
        "change-disposition" => Ok(ObjectKind::ChangeDisposition),
        _ => Err(ParseIssue::malformed("object-kind")),
    }
}

fn evidence_role(value: &str) -> Result<EvidenceRole, ParseIssue> {
    match value {
        "positive" => Ok(EvidenceRole::Positive),
        "negative" => Ok(EvidenceRole::Negative),
        "property" => Ok(EvidenceRole::Property),
        "exhaustive-model" => Ok(EvidenceRole::ExhaustiveModel),
        "formal-property" => Ok(EvidenceRole::FormalProperty),
        "structural" => Ok(EvidenceRole::Structural),
        _ => Err(ParseIssue::malformed("evidence-role")),
    }
}

fn authority_expectation(value: &str) -> Result<AuthorityExpectation, ParseIssue> {
    match value {
        "forbidden" => Ok(AuthorityExpectation::Forbidden),
        "optional" => Ok(AuthorityExpectation::Optional),
        "required" => Ok(AuthorityExpectation::Required),
        _ => Err(ParseIssue::malformed("authority-expectation")),
    }
}

fn surface_disposition(value: &str) -> Result<SurfaceDispositionKind, ParseIssue> {
    match value {
        "normative" => Ok(SurfaceDispositionKind::Normative),
        "prohibited" => Ok(SurfaceDispositionKind::Prohibited),
        "deferred" => Ok(SurfaceDispositionKind::Deferred),
        "not-applicable" => Ok(SurfaceDispositionKind::NotApplicable),
        _ => Err(ParseIssue::malformed("surface-disposition")),
    }
}

fn enforcement_mode(value: &str) -> Result<EnforcementMode, ParseIssue> {
    match value {
        "advisory" => Ok(EnforcementMode::Advisory),
        "blocking" => Ok(EnforcementMode::Blocking),
        _ => Err(ParseIssue::malformed("enforcement")),
    }
}

fn policy_lifecycle(value: &str) -> Result<PolicyLifecycle, ParseIssue> {
    match value {
        "active" => Ok(PolicyLifecycle::Active),
        "superseded" => Ok(PolicyLifecycle::Superseded),
        "retired" => Ok(PolicyLifecycle::Retired),
        _ => Err(ParseIssue::malformed("policy-lifecycle")),
    }
}

fn debt_lifecycle(value: &str) -> Result<DebtLifecycle, ParseIssue> {
    match value {
        "active" => Ok(DebtLifecycle::Active),
        "discharged" => Ok(DebtLifecycle::Discharged),
        "superseded" => Ok(DebtLifecycle::Superseded),
        _ => Err(ParseIssue::malformed("debt-lifecycle")),
    }
}

fn waiver_lifecycle(value: &str) -> Result<WaiverLifecycle, ParseIssue> {
    match value {
        "active" => Ok(WaiverLifecycle::Active),
        "discharged" => Ok(WaiverLifecycle::Discharged),
        "revoked" => Ok(WaiverLifecycle::Revoked),
        "superseded" => Ok(WaiverLifecycle::Superseded),
        _ => Err(ParseIssue::malformed("waiver-lifecycle")),
    }
}

fn navigation_kind(value: &str) -> Result<NavigationKind, ParseIssue> {
    match value {
        "implements" => Ok(NavigationKind::Implements),
        "owns" => Ok(NavigationKind::Owns),
        "consumes" => Ok(NavigationKind::Consumes),
        "configures" => Ok(NavigationKind::Configures),
        _ => Err(ParseIssue::malformed("navigation")),
    }
}

fn derivation_relation(value: &str) -> Result<DerivationRelation, ParseIssue> {
    match value {
        "byte-equal" => Ok(DerivationRelation::ByteEqual),
        "generated-from" => Ok(DerivationRelation::GeneratedFrom),
        "structurally-equivalent" => Ok(DerivationRelation::StructurallyEquivalent),
        "host-consistency" => Ok(DerivationRelation::HostConsistency),
        _ => Err(ParseIssue::malformed("derivation-relation")),
    }
}

fn obligation_kind(value: &str) -> Result<ObligationKind, ParseIssue> {
    match value {
        "authority" => Ok(ObligationKind::Authority),
        "surface-completeness" => Ok(ObligationKind::SurfaceCompleteness),
        "surface-authority" => Ok(ObligationKind::SurfaceAuthority),
        "required-role" => Ok(ObligationKind::RequiredRole),
        "freshness" => Ok(ObligationKind::Freshness),
        "oracle-selection" => Ok(ObligationKind::OracleSelection),
        "execution" => Ok(ObligationKind::Execution),
        "derivation-consistency" => Ok(ObligationKind::DerivationConsistency),
        "proof" => Ok(ObligationKind::Proof),
        "change-impact" => Ok(ObligationKind::ChangeImpact),
        _ => Err(ParseIssue::malformed("obligation-kind")),
    }
}

fn verdict(value: &str) -> Result<Verdict, ParseIssue> {
    match value {
        "not-run" => Ok(Verdict::NotRun),
        "pass" => Ok(Verdict::Pass),
        "fail" => Ok(Verdict::Fail),
        "error" => Ok(Verdict::Error),
        "timeout" => Ok(Verdict::Timeout),
        _ => Err(ParseIssue::malformed("verdict")),
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct AtomDef {
    id: String,
    state: String,
    kind: String,
    scope: Vec<String>,
    statement: String,
    approval: Option<ApprovalDef>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct PayloadDef {
    schema: String,
    bytes_hex: String,
}

impl PayloadDef {
    fn build(self) -> Result<TypedPayload, ParseIssue> {
        TypedPayload::new(parse_schema_id(&self.schema)?, decode_hex(&self.bytes_hex)?)
            .map_err(core_issue)
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct AtomRefDef {
    id: String,
    revision: String,
}

impl AtomRefDef {
    fn build(self) -> Result<AtomRevisionRef, ParseIssue> {
        Ok(AtomRevisionRef::new(
            StableId::new(&self.id).map_err(id_issue)?,
            digest(&self.revision)?,
        ))
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ObjectRefDef {
    kind: String,
    id: String,
    digest: String,
}

impl ObjectRefDef {
    fn build(self) -> Result<ObjectRef, ParseIssue> {
        Ok(ObjectRef::new(
            object_kind(&self.kind)?,
            StableId::new(&self.id).map_err(id_issue)?,
            digest(&self.digest)?,
        ))
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ApprovalDef {
    authority: String,
    record: String,
    digest: String,
}

impl ApprovalDef {
    fn build(self) -> Result<ApprovalBinding, ParseIssue> {
        ApprovalBinding::new(
            parse_scope_id(&self.authority)?,
            RepositoryPath::new(&self.record)
                .map_err(|error| ParseIssue::malformed(error.code()))?,
            digest(&self.digest)?,
        )
        .map_err(core_issue)
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ToolDef {
    id: String,
    version: String,
    executable: String,
    configuration: String,
}

impl ToolDef {
    fn build(self) -> Result<ToolIdentity, ParseIssue> {
        ToolIdentity::new(
            parse_scope_id(&self.id)?,
            self.version,
            digest(&self.executable)?,
            digest(&self.configuration)?,
        )
        .map_err(core_issue)
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct SelectorDef {
    root: String,
    adapter_id: String,
    include_patterns: Vec<String>,
    exclude_patterns: Vec<String>,
    authority_expectation: String,
}

impl SelectorDef {
    fn build(self) -> Result<TypedSelector, ParseIssue> {
        TypedSelector::new(
            RepositoryPath::new(&self.root).map_err(|error| ParseIssue::malformed(error.code()))?,
            parse_schema_id(&self.adapter_id)?,
            self.include_patterns,
            self.exclude_patterns,
            authority_expectation(&self.authority_expectation)?,
        )
        .map_err(core_issue)
    }
}

#[derive(Deserialize)]
#[serde(tag = "kind", rename_all = "kebab-case", deny_unknown_fields)]
enum ArtifactDef {
    Object {
        object: ObjectRefDef,
    },
    RepositoryArtifact {
        path: String,
        raw_domain: String,
        media_type: String,
        representation: String,
        digest: String,
    },
}

impl ArtifactDef {
    fn build(self) -> Result<ArtifactBinding, ParseIssue> {
        match self {
            Self::Object { object } => object.build().map(ArtifactBinding::object),
            Self::RepositoryArtifact {
                path,
                raw_domain,
                media_type,
                representation,
                digest: value,
            } => ArtifactBinding::repository(
                RepositoryPath::new(&path).map_err(|error| ParseIssue::malformed(error.code()))?,
                raw_domain,
                media_type,
                representation,
                digest(&value)?,
            )
            .map_err(core_issue),
        }
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct DispositionDef {
    key: PayloadDef,
    disposition: String,
    payload: Option<PayloadDef>,
    authority: Option<AtomRefDef>,
}

impl DispositionDef {
    fn build(self) -> Result<SurfaceDisposition, ParseIssue> {
        SurfaceDisposition::new(
            self.key.build()?,
            surface_disposition(&self.disposition)?,
            self.payload.map(PayloadDef::build).transpose()?,
            self.authority.map(AtomRefDef::build).transpose()?,
        )
        .map_err(core_issue)
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct SurfaceDef {
    id: String,
    member_key_schema: PayloadDef,
    disposition_vocabulary: PayloadDef,
    member_keys: Vec<PayloadDef>,
    generation: u64,
    dispositions: Vec<DispositionDef>,
    policies: Vec<ObjectRefDef>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct PolicyDef {
    id: String,
    scope: PayloadDef,
    required_roles: Vec<String>,
    enforcement: String,
    ratchet_id: String,
    authority_selectors: Vec<SelectorDef>,
    derivation_requirements: Option<PayloadDef>,
    active_approval: ApprovalDef,
    lifecycle: String,
    replacements: Vec<ObjectRefDef>,
    terminal_approval: Option<ApprovalDef>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ObligationDef {
    policy_id: String,
    kind: String,
    subject: ObjectRefDef,
    required_role: Option<String>,
}

impl ObligationDef {
    fn build(self) -> Result<ObligationRef, ParseIssue> {
        ObligationRef::new(
            StableId::new(&self.policy_id).map_err(id_issue)?,
            obligation_kind(&self.kind)?,
            self.subject.build()?,
            self.required_role
                .as_deref()
                .map(evidence_role)
                .transpose()?,
        )
        .map_err(core_issue)
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct DebtDef {
    id: String,
    obligations: Vec<ObligationDef>,
    objects: Vec<ObjectRefDef>,
    rationale: String,
    owner: String,
    remediation: String,
    discharge_condition: PayloadDef,
    external_references: Vec<String>,
    lifecycle: String,
    discharge_evidence: Vec<ObjectRefDef>,
    replacements: Vec<ObjectRefDef>,
    terminal_approval: Option<ApprovalDef>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct WaiverDef {
    id: String,
    obligations: Vec<ObligationDef>,
    objects: Vec<ObjectRefDef>,
    rationale: String,
    owner: String,
    discharge_condition: PayloadDef,
    active_approval: ApprovalDef,
    external_references: Vec<String>,
    lifecycle: String,
    discharge_evidence: Vec<ObjectRefDef>,
    replacements: Vec<ObjectRefDef>,
    terminal_approval: Option<ApprovalDef>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ReaffirmationDef {
    id: String,
    affected_object_kind: String,
    affected_object_id: String,
    base_object_digest: String,
    head_object_digest: String,
    review_approval: ApprovalDef,
    review_statement: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct DerivationDef {
    id: String,
    sources: Vec<ArtifactDef>,
    targets: Vec<ArtifactDef>,
    tool: ToolDef,
    configuration: String,
    oracle: ObjectRefDef,
    relation: String,
    relation_scope: PayloadDef,
    exclusions: Vec<String>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct AssumptionDef {
    id: String,
    statement: String,
    scope: PayloadDef,
    exclusions: Vec<String>,
    approval: ApprovalDef,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct FiniteModelDef {
    id: String,
    state_space: PayloadDef,
    event_alphabet: PayloadDef,
    transition_relation: PayloadDef,
    initial_states: Vec<PayloadDef>,
    bound: PayloadDef,
    checked_invariants: Vec<AtomRefDef>,
    assumptions: Vec<ObjectRefDef>,
    exclusions: Vec<String>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct FormalPropertyDef {
    id: String,
    atoms: Vec<AtomRefDef>,
    property: PayloadDef,
    implementation_abstraction: PayloadDef,
    assumptions: Vec<ObjectRefDef>,
    exclusions: Vec<String>,
    correspondence_approval: ApprovalDef,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ProofRecordDef {
    id: String,
    atoms: Vec<AtomRefDef>,
    property: ObjectRefDef,
    implementation: ArtifactDef,
    assumptions: Vec<ObjectRefDef>,
    translators: Vec<ToolDef>,
    verifier: ToolDef,
    verifier_configuration: String,
    external_models: Vec<ArtifactDef>,
    generated: Vec<ArtifactDef>,
    proof_sources: Vec<ArtifactDef>,
    verdict: String,
    scope: PayloadDef,
    exclusions: Vec<String>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct CarrierDef {
    id: String,
    role: String,
    atoms: Vec<AtomRefDef>,
    oracle: ObjectRefDef,
    configuration: String,
    corpora: Vec<String>,
    scope: PayloadDef,
    assumptions: Vec<ObjectRefDef>,
    exclusions: Vec<String>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ImplementationDef {
    id: String,
    atoms: Vec<AtomRefDef>,
    navigation: String,
}

/// Direct unit coverage for private helpers that no adapter entrypoint can
/// exercise: `parse_markdown_semantic`'s catch-all is unreachable because
/// `markdown.rs` filters unsupported fences before calling it; `atom_issue`'s
/// pass-through arm cannot be produced by any value that survives
/// `decode_atom`'s own guards; and `classify_toml_error`'s "duplicate field"
/// wording is a forward-compatible synonym the pinned `toml` release never
/// emits for "duplicate key" (see `matches_duplicate_field_wording`).
#[cfg(test)]
#[allow(clippy::expect_used)]
mod tests {
    use super::{
        atom_issue, classify_toml_error, hex_nibble, parse_carrier, parse_implementation_link,
        parse_markdown_semantic,
    };
    use buoy_core::SemanticDigest;

    #[test]
    fn parse_markdown_semantic_rejects_an_unsupported_syntax_version() {
        let issue = parse_markdown_semantic("spec-provenance-unsupported-v9", "")
            .expect_err("an unrecognized syntax version must be rejected");
        assert_eq!(issue.code, "SPG-ADAPTER-UNSUPPORTED-SYNTAX-VERSION");
        assert_eq!(issue.context, "spec-provenance-unsupported-v9");
    }

    #[test]
    fn parse_markdown_semantic_reclassifies_a_malformed_atom_toml_body() {
        let issue = parse_markdown_semantic("spec-provenance-atom-v1", "not = [toml")
            .expect_err("invalid TOML syntax in an atom body must be rejected");
        assert_eq!(issue.code, "SPG-ADAPTER-MALFORMED-ATOM");
    }

    #[test]
    fn atom_issue_passes_through_a_field_shaped_issue_unchanged() {
        for code in [
            "SPG-ADAPTER-UNSUPPORTED-FIELD",
            "SPG-ADAPTER-MISSING-FIELD",
            "SPG-ADAPTER-DUPLICATE-FIELD",
        ] {
            let issue = super::ParseIssue {
                code,
                context: String::from("field-context"),
            };
            let mapped = atom_issue(issue.clone());
            assert_eq!(mapped, issue);
        }
    }

    #[test]
    fn atom_issue_reclassifies_a_non_field_issue_as_malformed_atom() {
        let issue = super::ParseIssue {
            code: "SPG-ADAPTER-MALFORMED-OBJECT",
            context: String::from("object-context"),
        };
        let mapped = atom_issue(issue);
        assert_eq!(mapped.code, "SPG-ADAPTER-MALFORMED-ATOM");
        assert_eq!(mapped.context, "object-context");
    }

    #[test]
    fn hex_nibble_covers_every_digit_and_letter_value() {
        assert_eq!(hex_nibble(b'0'), 0);
        assert_eq!(hex_nibble(b'9'), 9);
        assert_eq!(hex_nibble(b'a'), 10);
        assert_eq!(hex_nibble(b'f'), 15);
    }

    #[test]
    fn classify_toml_error_recognizes_the_duplicate_field_synonym() {
        let error = <toml::de::Error as serde::de::Error>::custom("duplicate field: `id`");
        let issue = classify_toml_error(&error);
        assert_eq!(issue.code, "SPG-ADAPTER-DUPLICATE-FIELD");
        assert_eq!(issue.context, "duplicate-field");
    }

    #[test]
    fn rust_object_parsers_project_final_construction_and_identifier_errors() {
        let oracle_digest = SemanticDigest::xxh3_128([2; 16]);
        let configuration_digest = SemanticDigest::xxh3_128([3; 16]);
        let carrier = format!(
            r#"
id = "CAR-CONTROL"
role = "positive"
atoms = []
oracle = {{ kind = "oracle", id = "ORACLE-CONTROL", digest = "{oracle_digest}" }}
configuration = "{configuration_digest}"
corpora = []
scope = {{ schema = "spec-provenance.scope.exact-corpus/v1", bytes_hex = "61" }}
assumptions = []
exclusions = []
"#
        );
        assert!(parse_carrier(&carrier, SemanticDigest::xxh3_128([1; 16]), None).is_err());
        let revision = SemanticDigest::xxh3_128([1; 16]);
        let implementation = format!(
            r#"
id = "invalid id"
atoms = [{{ id = "ATOM-001", revision = "{revision}" }}]
navigation = "implements"
"#
        );
        assert!(
            parse_implementation_link(&implementation, SemanticDigest::xxh3_128([1; 16])).is_err()
        );
    }

    #[test]
    fn every_markdown_object_parser_rejects_an_invalid_top_level_id() {
        let fixture = include_str!("../../fixtures/adapters/markdown/objects.md");
        let mut blocks = fixture.split("```").skip(1);
        while let Some(block) = blocks.next() {
            let (syntax, body) = block.split_once('\n').expect("fenced object");
            let invalid = body.replacen("id = \"", "id = \"invalid ", 1);
            assert!(
                parse_markdown_semantic(syntax, &invalid).is_err(),
                "{syntax}"
            );
            let _ = blocks.next();
        }
    }

    #[test]
    #[allow(clippy::too_many_lines)]
    fn nested_schema_builders_reject_invalid_boundary_identifiers_and_paths() {
        let digest = SemanticDigest::xxh3_128([1; 16]).canonical_text();
        assert!(super::parse_schema_id("invalid schema").is_err());
        assert!(super::parse_scope_id("invalid scope").is_err());
        assert!(
            super::PayloadDef {
                schema: String::from("invalid schema"),
                bytes_hex: String::from("00"),
            }
            .build()
            .is_err()
        );
        assert!(
            super::AtomRefDef {
                id: String::from("invalid id"),
                revision: digest.clone(),
            }
            .build()
            .is_err()
        );
        assert!(
            super::ObjectRefDef {
                kind: String::from("atom"),
                id: String::from("invalid id"),
                digest: digest.clone(),
            }
            .build()
            .is_err()
        );
        assert!(
            super::ApprovalDef {
                authority: String::from("maintainers"),
                record: String::from("../invalid"),
                digest: digest.clone(),
            }
            .build()
            .is_err()
        );
        assert!(
            super::ApprovalDef {
                authority: String::from("invalid scope"),
                record: String::from("reviews/control.md"),
                digest: digest.clone(),
            }
            .build()
            .is_err()
        );
        assert!(
            super::ToolDef {
                id: String::from("invalid scope"),
                version: String::from("1.0.0"),
                executable: digest.clone(),
                configuration: digest.clone(),
            }
            .build()
            .is_err()
        );
        assert!(
            super::SelectorDef {
                root: String::from("../invalid"),
                adapter_id: String::from("markdown/v1"),
                include_patterns: Vec::new(),
                exclude_patterns: Vec::new(),
                authority_expectation: String::from("optional"),
            }
            .build()
            .is_err()
        );
        assert!(
            super::SelectorDef {
                root: String::from("spec"),
                adapter_id: String::from("invalid schema"),
                include_patterns: Vec::new(),
                exclude_patterns: Vec::new(),
                authority_expectation: String::from("optional"),
            }
            .build()
            .is_err()
        );
        assert!(
            super::ArtifactDef::RepositoryArtifact {
                path: String::from("../invalid"),
                raw_domain: String::from("control"),
                media_type: String::from("text/plain"),
                representation: String::from("saved-bytes"),
                digest: digest.clone(),
            }
            .build()
            .is_err()
        );
        assert!(
            super::ObligationDef {
                policy_id: String::from("invalid id"),
                kind: String::from("positive"),
                subject: super::ObjectRefDef {
                    kind: String::from("atom"),
                    id: String::from("ATOM-001"),
                    digest,
                },
                required_role: None,
            }
            .build()
            .is_err()
        );

        let atom = r#"
id = "ATOM-CONTROL"
state = "proposed"
kind = "behavioral"
scope = ["invalid scope"]
statement = "The adapter MUST reject an invalid scope."
"#;
        assert!(parse_markdown_semantic("spec-provenance-atom-v1", atom).is_err());

        let fixture = include_str!("../../fixtures/adapters/markdown/objects.md");
        let body = |syntax: &str| {
            fixture
                .split("```")
                .find_map(|block| {
                    let (candidate, body) = block.split_once('\n')?;
                    (candidate == syntax).then(|| body.to_owned())
                })
                .expect("fixture object")
        };
        let policy = body("spec-provenance-policy-v1").replace(
            "required_roles = [\"positive\"]",
            "required_roles = [\"positive\", \"positive\"]",
        );
        assert!(parse_markdown_semantic("spec-provenance-policy-v1", &policy).is_err());
        let debt = body("spec-provenance-debt-v1")
            .replace("owner = \"adapter-owner\"", "owner = \"invalid scope\"");
        assert!(parse_markdown_semantic("spec-provenance-debt-v1", &debt).is_err());
        let waiver = body("spec-provenance-waiver-v1")
            .replace("owner = \"adapter-owner\"", "owner = \"invalid scope\"");
        assert!(parse_markdown_semantic("spec-provenance-waiver-v1", &waiver).is_err());
        let reaffirmation = body("spec-provenance-reaffirmation-v1").replace(
            "affected_object_id = \"ATOM-ADAPTER-001\"",
            "affected_object_id = \"invalid id\"",
        );
        assert!(
            parse_markdown_semantic("spec-provenance-reaffirmation-v1", &reaffirmation).is_err()
        );
    }
}
