//! Mapping from frozen Chelis records into repository-independent core values.

use crate::records::ChelisRecord;
use buoy::core::{
    ActiveAlgorithm, ApprovalBinding, ArtifactBinding, Atom, AtomKind, AtomRevisionRef,
    AuthorityExpectation, CoveragePolicy, CoveragePolicyRevision, DebtLifecycle, DebtRecord,
    DecisionSurface, DecisionSurfaceDomain, Derivation, DerivationRelation, EnforcementMode,
    EvidenceCarrier, EvidenceRole, FormalProperty, ImplementationLink, LifecycleState,
    NavigationKind, ObjectKind, ObjectRef, ObligationKind, ObligationRef, Oracle, OracleKind,
    PolicyLifecycle, Reaffirmation, RepositoryPath, SchemaId, ScopeId, SemanticDigest, StableId,
    SurfaceDisposition, SurfaceDispositionKind, ToolIdentity, TypedPayload, TypedSelector, Waiver,
};

/// One mapped repository-independent core value.
#[derive(Clone, Debug, PartialEq)]
pub enum MappedValue {
    /// One authoritative requirement atom.
    Atom(Atom),
    /// One exact evidence carrier.
    Carrier(EvidenceCarrier),
    /// One navigational implementation link.
    Link(ImplementationLink),
    /// One complete governed decision surface.
    Surface(DecisionSurface),
    /// One coverage policy.
    Policy(CoveragePolicy),
    /// One debt record.
    Debt(DebtRecord),
    /// One process waiver.
    Waiver(Waiver),
    /// One review-only reaffirmation.
    Reaffirmation(Reaffirmation),
    /// One formal property declaration.
    FormalProperty(FormalProperty),
    /// One intentional derivation.
    Derivation(Derivation),
    /// One effectful oracle declaration.
    Oracle(Oracle),
}

impl MappedValue {
    /// Closed graph object kind of this value.
    #[must_use]
    pub const fn object_kind(&self) -> ObjectKind {
        match self {
            Self::Atom(_) => ObjectKind::Atom,
            Self::Carrier(_) => ObjectKind::EvidenceCarrier,
            Self::Link(_) => ObjectKind::ImplementationLink,
            Self::Surface(_) => ObjectKind::DecisionSurface,
            Self::Policy(_) => ObjectKind::CoveragePolicy,
            Self::Debt(_) => ObjectKind::DebtRecord,
            Self::Waiver(_) => ObjectKind::Waiver,
            Self::Reaffirmation(_) => ObjectKind::Reaffirmation,
            Self::FormalProperty(_) => ObjectKind::FormalProperty,
            Self::Derivation(_) => ObjectKind::Derivation,
            Self::Oracle(_) => ObjectKind::Oracle,
        }
    }
}

/// Deterministic mapping failure with schema-owned context.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MapError {
    /// Deterministic failure context.
    pub context: String,
}

impl MapError {
    fn new(context: impl Into<String>) -> Self {
        Self {
            context: context.into(),
        }
    }
}

struct Fields<'a> {
    record: &'a ChelisRecord,
}

impl Fields<'_> {
    fn text(&self, key: &str) -> Result<&str, MapError> {
        self.record
            .fields
            .get(key)
            .map(String::as_str)
            .filter(|value| !value.is_empty())
            .ok_or_else(|| MapError::new(format!("missing-field:{key}")))
    }

    fn optional(&self, key: &str) -> Option<&str> {
        self.record
            .fields
            .get(key)
            .map(String::as_str)
            .filter(|value| !value.is_empty())
    }

    fn list(&self, key: &str) -> Vec<&str> {
        self.optional(key)
            .map(|value| value.split(';').map(str::trim).collect())
            .unwrap_or_default()
    }

    fn required_list(&self, key: &str) -> Result<Vec<&str>, MapError> {
        let values = self.list(key);
        if values.is_empty() {
            return Err(MapError::new(format!("missing-field:{key}")));
        }
        Ok(values)
    }

    fn id(&self, key: &str) -> Result<StableId, MapError> {
        StableId::new(self.text(key)?).map_err(|_| MapError::new(format!("invalid-id:{key}")))
    }

    fn scope(&self, key: &str) -> Result<ScopeId, MapError> {
        ScopeId::new(self.text(key)?).map_err(|_| MapError::new(format!("invalid-scope:{key}")))
    }

    fn digest(&self, key: &str) -> Result<SemanticDigest, MapError> {
        parse_digest(self.text(key)?).map_err(|()| MapError::new(format!("invalid-digest:{key}")))
    }

    fn number(&self, key: &str) -> Result<u64, MapError> {
        self.text(key)?
            .parse::<u64>()
            .map_err(|_| MapError::new(format!("invalid-number:{key}")))
    }

    fn payload(&self, schema_key: &str, value_key: &str) -> Result<TypedPayload, MapError> {
        let schema = SchemaId::new(self.text(schema_key)?)
            .map_err(|_| MapError::new(format!("invalid-schema:{schema_key}")))?;
        TypedPayload::new(schema, self.text(value_key)?.as_bytes().to_vec())
            .map_err(|_| MapError::new(format!("invalid-payload:{value_key}")))
    }

    fn approval(&self) -> Result<ApprovalBinding, MapError> {
        ApprovalBinding::new(
            self.scope("approval-authority")?,
            RepositoryPath::new(self.text("approval-record")?)
                .map_err(|_| MapError::new("invalid-path:approval-record"))?,
            self.digest("approval-digest")?,
        )
        .map_err(|_| MapError::new("invalid-approval"))
    }

    fn atoms(&self) -> Result<Vec<AtomRevisionRef>, MapError> {
        self.required_list("atoms")?
            .into_iter()
            .map(parse_atom_revision)
            .collect()
    }

    fn oracle_reference(&self) -> Result<ObjectRef, MapError> {
        Ok(ObjectRef::new(
            ObjectKind::Oracle,
            self.id("oracle-id")?,
            self.digest("oracle-digest")?,
        ))
    }

    fn exclusions(&self) -> Vec<String> {
        self.list("exclusions")
            .into_iter()
            .map(str::to_owned)
            .collect()
    }

    fn tool(&self, prefix: &str) -> Result<ToolIdentity, MapError> {
        ToolIdentity::new(
            ScopeId::new(self.text(&format!("{prefix}-id"))?)
                .map_err(|_| MapError::new(format!("invalid-scope:{prefix}-id")))?,
            self.text(&format!("{prefix}-version"))?,
            self.digest(&format!("{prefix}-digest"))?,
            self.digest(&format!("{prefix}-configuration-digest"))?,
        )
        .map_err(|_| MapError::new(format!("invalid-tool:{prefix}")))
    }
}

fn parse_digest(value: &str) -> Result<SemanticDigest, ()> {
    SemanticDigest::parse_canonical(value).map_err(|_| ())
}

fn parse_atom_revision(value: &str) -> Result<AtomRevisionRef, MapError> {
    let (id, digest) = value
        .split_once('@')
        .ok_or_else(|| MapError::new(format!("invalid-atom-revision:{value}")))?;
    Ok(AtomRevisionRef::new(
        StableId::new(id).map_err(|_| MapError::new(format!("invalid-atom-revision:{value}")))?,
        parse_digest(digest)
            .map_err(|()| MapError::new(format!("invalid-atom-revision:{value}")))?,
    ))
}

fn parse_object_kind(value: &str) -> Result<ObjectKind, MapError> {
    match value {
        "atom" => Ok(ObjectKind::Atom),
        "evidence-carrier" => Ok(ObjectKind::EvidenceCarrier),
        "decision-surface" => Ok(ObjectKind::DecisionSurface),
        "implementation-link" => Ok(ObjectKind::ImplementationLink),
        "coverage-policy" => Ok(ObjectKind::CoveragePolicy),
        "derivation" => Ok(ObjectKind::Derivation),
        "debt-record" => Ok(ObjectKind::DebtRecord),
        "waiver" => Ok(ObjectKind::Waiver),
        "oracle" => Ok(ObjectKind::Oracle),
        "formal-property" => Ok(ObjectKind::FormalProperty),
        "reaffirmation" => Ok(ObjectKind::Reaffirmation),
        _ => Err(MapError::new(format!("invalid-object-kind:{value}"))),
    }
}

fn parse_object_reference(value: &str) -> Result<ObjectRef, MapError> {
    let (kind, rest) = value
        .split_once(':')
        .ok_or_else(|| MapError::new(format!("invalid-object-reference:{value}")))?;
    let (id, digest) = rest
        .split_once('@')
        .ok_or_else(|| MapError::new(format!("invalid-object-reference:{value}")))?;
    Ok(ObjectRef::new(
        parse_object_kind(kind)?,
        StableId::new(id)
            .map_err(|_| MapError::new(format!("invalid-object-reference:{value}")))?,
        parse_digest(digest)
            .map_err(|()| MapError::new(format!("invalid-object-reference:{value}")))?,
    ))
}

fn parse_role(value: &str) -> Result<EvidenceRole, MapError> {
    match value {
        "positive" => Ok(EvidenceRole::Positive),
        "negative" => Ok(EvidenceRole::Negative),
        "property" => Ok(EvidenceRole::Property),
        "exhaustive-model" => Ok(EvidenceRole::ExhaustiveModel),
        "formal-property" => Ok(EvidenceRole::FormalProperty),
        "structural" => Ok(EvidenceRole::Structural),
        _ => Err(MapError::new(format!("invalid-role:{value}"))),
    }
}

fn parse_obligation_kind(value: &str) -> Result<ObligationKind, MapError> {
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
        _ => Err(MapError::new(format!("invalid-obligation-kind:{value}"))),
    }
}

fn parse_obligations(fields: &Fields<'_>) -> Result<Vec<ObligationRef>, MapError> {
    fields
        .required_list("obligations")?
        .into_iter()
        .map(|entry| {
            let mut parts = entry.splitn(4, ':');
            let policy = parts
                .next()
                .ok_or_else(|| MapError::new(format!("invalid-obligation:{entry}")))?;
            let kind = parts
                .next()
                .ok_or_else(|| MapError::new(format!("invalid-obligation:{entry}")))?;
            let subject = parts
                .next()
                .zip(parts.next())
                .map(|(kind_text, reference)| format!("{kind_text}:{reference}"))
                .ok_or_else(|| MapError::new(format!("invalid-obligation:{entry}")))?;
            let kind = parse_obligation_kind(kind)?;
            let (subject, role) = match subject.rsplit_once("!role=") {
                Some((reference, role)) => (reference.to_owned(), Some(parse_role(role)?)),
                None => (subject, None),
            };
            ObligationRef::new(
                StableId::new(policy)
                    .map_err(|_| MapError::new(format!("invalid-obligation:{entry}")))?,
                kind,
                parse_object_reference(&subject)?,
                role,
            )
            .map_err(|_| MapError::new(format!("invalid-obligation:{entry}")))
        })
        .collect()
}

fn parse_artifacts(fields: &Fields<'_>, key: &str) -> Result<Vec<ArtifactBinding>, MapError> {
    fields
        .required_list(key)?
        .into_iter()
        .map(|entry| {
            let parts = entry.split('|').collect::<Vec<_>>();
            let [path, media, representation, digest] = parts.as_slice() else {
                return Err(MapError::new(format!("invalid-artifact:{entry}")));
            };
            ArtifactBinding::repository(
                RepositoryPath::new(path)
                    .map_err(|_| MapError::new(format!("invalid-artifact:{entry}")))?,
                "raw-artifact",
                *media,
                *representation,
                parse_digest(digest)
                    .map_err(|()| MapError::new(format!("invalid-artifact:{entry}")))?,
            )
            .map_err(|_| MapError::new(format!("invalid-artifact:{entry}")))
        })
        .collect()
}

fn map_authority(fields: &Fields<'_>) -> Result<MappedValue, MapError> {
    map_authority_with(None, fields)
}

fn map_authority_with(
    active: Option<ActiveAlgorithm>,
    fields: &Fields<'_>,
) -> Result<MappedValue, MapError> {
    let kind = match fields.text("kind")? {
        "behavioral" => AtomKind::Behavioral,
        "structural" => AtomKind::Structural,
        "constraint" => AtomKind::Constraint,
        other => return Err(MapError::new(format!("invalid-atom-kind:{other}"))),
    };
    let scopes = fields
        .required_list("scopes")?
        .into_iter()
        .map(|scope| ScopeId::new(scope).map_err(|_| MapError::new("invalid-scope:scopes")))
        .collect::<Result<Vec<_>, _>>()?;
    let (state, approval) = match fields.optional("lifecycle").unwrap_or("proposed") {
        "proposed" => (LifecycleState::Proposed, None),
        "ratified" => (LifecycleState::Ratified, Some(fields.approval()?)),
        other => return Err(MapError::new(format!("invalid-lifecycle:{other}"))),
    };
    let id = fields.id("id")?;
    let statement = fields.text("statement")?;
    match active {
        Some(active) => Atom::new_with(active, id, state, kind, scopes, statement, approval),
        None => Atom::new(id, state, kind, scopes, statement, approval),
    }
    .map(MappedValue::Atom)
    .map_err(|_| MapError::new("invalid-atom"))
}

/// Maps one authority record under an explicit active digest member.
///
/// Compact bindings are content addressed under the member their address
/// names, so their atom revisions must come from that same member rather
/// than the build default.
///
/// # Errors
///
/// Returns the same deterministic contexts [`map_record`] returns.
pub fn map_atom_with(
    active: ActiveAlgorithm,
    record: &ChelisRecord,
) -> Result<MappedValue, MapError> {
    if record.kind != "authority" {
        return Err(MapError::new(format!(
            "unknown-record-kind:{}",
            record.kind
        )));
    }
    map_authority_with(Some(active), &Fields { record })
}

fn map_carrier(fields: &Fields<'_>) -> Result<MappedValue, MapError> {
    let corpora = fields
        .list("corpora")
        .into_iter()
        .map(|value| {
            parse_digest(value).map_err(|()| MapError::new(format!("invalid-digest:{value}")))
        })
        .collect::<Result<Vec<_>, _>>()?;
    EvidenceCarrier::new(
        fields.id("id")?,
        parse_role(fields.text("role")?)?,
        fields.atoms()?,
        fields.digest("item-digest")?,
        fields.oracle_reference()?,
        fields.digest("configuration-digest")?,
        corpora,
        fields.payload("scope-schema", "scope")?,
        Vec::new(),
        fields.exclusions(),
    )
    .map(MappedValue::Carrier)
    .map_err(|_| MapError::new("invalid-carrier"))
}

fn map_link(fields: &Fields<'_>) -> Result<MappedValue, MapError> {
    let navigation = match fields.text("navigation")? {
        "implements" => NavigationKind::Implements,
        "owns" => NavigationKind::Owns,
        "consumes" => NavigationKind::Consumes,
        "configures" => NavigationKind::Configures,
        other => return Err(MapError::new(format!("invalid-navigation:{other}"))),
    };
    ImplementationLink::new(
        fields.id("id")?,
        fields.atoms()?,
        fields.digest("item-digest")?,
        navigation,
    )
    .map(MappedValue::Link)
    .map_err(|_| MapError::new("invalid-link"))
}

fn map_surface(fields: &Fields<'_>) -> Result<MappedValue, MapError> {
    let member_schema = SchemaId::new(fields.text("member-key-schema")?)
        .map_err(|_| MapError::new("invalid-schema:member-key-schema"))?;
    let disposition_schema = SchemaId::new(fields.text("disposition-schema")?)
        .map_err(|_| MapError::new("invalid-schema:disposition-schema"))?;
    let member_payload = |member: &str| {
        TypedPayload::new(member_schema.clone(), member.as_bytes().to_vec())
            .map_err(|_| MapError::new(format!("invalid-member:{member}")))
    };
    let members = fields
        .required_list("members")?
        .into_iter()
        .map(member_payload)
        .collect::<Result<Vec<_>, _>>()?;
    let domain = DecisionSurfaceDomain::new(
        fields.id("id")?,
        TypedPayload::new(member_schema.clone(), Vec::new())
            .map_err(|_| MapError::new("invalid-schema:member-key-schema"))?,
        TypedPayload::new(disposition_schema.clone(), Vec::new())
            .map_err(|_| MapError::new("invalid-schema:disposition-schema"))?,
        members,
        fields.number("generation")?,
    )
    .map_err(|_| MapError::new("invalid-surface-domain"))?;
    let rows = fields
        .required_list("rows")?
        .into_iter()
        .map(|row| {
            let mut parts = row.splitn(3, ':');
            let member = parts
                .next()
                .ok_or_else(|| MapError::new(format!("invalid-row:{row}")))?;
            let disposition = match parts.next() {
                Some("normative") => SurfaceDispositionKind::Normative,
                Some("prohibited") => SurfaceDispositionKind::Prohibited,
                Some("deferred") => SurfaceDispositionKind::Deferred,
                Some("not-applicable") => SurfaceDispositionKind::NotApplicable,
                _ => return Err(MapError::new(format!("invalid-row:{row}"))),
            };
            let authority = parts.next().map(parse_atom_revision).transpose()?;
            let payload = if disposition == SurfaceDispositionKind::NotApplicable {
                None
            } else {
                Some(
                    TypedPayload::new(disposition_schema.clone(), member.as_bytes().to_vec())
                        .map_err(|_| MapError::new(format!("invalid-row:{row}")))?,
                )
            };
            SurfaceDisposition::new(member_payload(member)?, disposition, payload, authority)
                .map_err(|_| MapError::new(format!("invalid-row:{row}")))
        })
        .collect::<Result<Vec<_>, _>>()?;
    DecisionSurface::new(domain, rows, Vec::new())
        .map(MappedValue::Surface)
        .map_err(|_| MapError::new("invalid-surface"))
}

fn map_policy(fields: &Fields<'_>) -> Result<MappedValue, MapError> {
    let required_roles = fields
        .required_list("required-roles")?
        .into_iter()
        .map(parse_role)
        .collect::<Result<Vec<_>, _>>()?;
    let enforcement = match fields.text("enforcement")? {
        "advisory" => EnforcementMode::Advisory,
        "blocking" => EnforcementMode::Blocking,
        other => return Err(MapError::new(format!("invalid-enforcement:{other}"))),
    };
    let expectation = match fields
        .optional("selector-expectation")
        .unwrap_or("optional")
    {
        "forbidden" => AuthorityExpectation::Forbidden,
        "optional" => AuthorityExpectation::Optional,
        "required" => AuthorityExpectation::Required,
        other => return Err(MapError::new(format!("invalid-expectation:{other}"))),
    };
    let selector = TypedSelector::new(
        RepositoryPath::new(fields.text("selector-root")?)
            .map_err(|_| MapError::new("invalid-path:selector-root"))?,
        SchemaId::new(fields.text("selector-adapter")?)
            .map_err(|_| MapError::new("invalid-schema:selector-adapter"))?,
        fields
            .required_list("selector-includes")?
            .into_iter()
            .map(str::to_owned)
            .collect(),
        fields
            .list("selector-excludes")
            .into_iter()
            .map(str::to_owned)
            .collect(),
        expectation,
    )
    .map_err(|_| MapError::new("invalid-selector"))?;
    let revision = CoveragePolicyRevision::new(
        fields.payload("scope-schema", "scope")?,
        required_roles,
        enforcement,
        fields.scope("ratchet")?,
        vec![selector],
        None,
    )
    .map_err(|_| MapError::new("invalid-policy-revision"))?;
    CoveragePolicy::new(
        fields.id("id")?,
        revision,
        fields.approval()?,
        PolicyLifecycle::Active,
        Vec::new(),
        None,
        Vec::new(),
    )
    .map(MappedValue::Policy)
    .map_err(|_| MapError::new("invalid-policy"))
}

fn map_debt(fields: &Fields<'_>) -> Result<MappedValue, MapError> {
    DebtRecord::new(
        fields.id("id")?,
        parse_obligations(fields)?,
        fields
            .list("objects")
            .into_iter()
            .map(parse_object_reference)
            .collect::<Result<Vec<_>, _>>()?,
        fields.text("rationale")?,
        fields.scope("owner")?,
        fields.text("remediation")?,
        fields.payload("discharge-schema", "discharge")?,
        fields
            .list("external")
            .into_iter()
            .map(str::to_owned)
            .collect(),
        DebtLifecycle::Active,
        Vec::new(),
        Vec::new(),
        None,
    )
    .map(MappedValue::Debt)
    .map_err(|_| MapError::new("invalid-debt"))
}

fn map_waiver(fields: &Fields<'_>) -> Result<MappedValue, MapError> {
    Waiver::new(
        fields.id("id")?,
        parse_obligations(fields)?,
        fields
            .list("objects")
            .into_iter()
            .map(parse_object_reference)
            .collect::<Result<Vec<_>, _>>()?,
        fields.text("rationale")?,
        fields.scope("owner")?,
        fields.payload("discharge-schema", "discharge")?,
        fields.approval()?,
        fields
            .list("external")
            .into_iter()
            .map(str::to_owned)
            .collect(),
        buoy::core::WaiverLifecycle::Active,
        Vec::new(),
        Vec::new(),
        None,
    )
    .map(MappedValue::Waiver)
    .map_err(|_| MapError::new("invalid-waiver"))
}

fn map_reaffirmation(fields: &Fields<'_>) -> Result<MappedValue, MapError> {
    Reaffirmation::new(
        fields.id("id")?,
        parse_object_kind(fields.text("affected-kind")?)?,
        fields.id("affected-id")?,
        fields.digest("base-digest")?,
        fields.digest("head-digest")?,
        fields.approval()?,
        fields.text("statement")?,
    )
    .map(MappedValue::Reaffirmation)
    .map_err(|_| MapError::new("invalid-reaffirmation"))
}

fn map_formal_property(fields: &Fields<'_>) -> Result<MappedValue, MapError> {
    FormalProperty::new(
        fields.id("id")?,
        fields.atoms()?,
        fields.payload("property-schema", "property")?,
        fields.payload("abstraction-schema", "abstraction")?,
        Vec::new(),
        fields.exclusions(),
        fields.approval()?,
    )
    .map(MappedValue::FormalProperty)
    .map_err(|_| MapError::new("invalid-formal-property"))
}

fn map_derivation(fields: &Fields<'_>) -> Result<MappedValue, MapError> {
    let relation = match fields.text("relation")? {
        "byte-equal" => DerivationRelation::ByteEqual,
        "generated-from" => DerivationRelation::GeneratedFrom,
        "structurally-equivalent" => DerivationRelation::StructurallyEquivalent,
        "host-consistency" => DerivationRelation::HostConsistency,
        other => return Err(MapError::new(format!("invalid-relation:{other}"))),
    };
    Derivation::new(
        fields.id("id")?,
        parse_artifacts(fields, "sources")?,
        parse_artifacts(fields, "targets")?,
        fields.tool("tool")?,
        fields.digest("configuration-digest")?,
        fields.oracle_reference()?,
        relation,
        fields.payload("scope-schema", "scope")?,
        fields.exclusions(),
    )
    .map(MappedValue::Derivation)
    .map_err(|_| MapError::new("invalid-derivation"))
}

fn map_oracle(fields: &Fields<'_>) -> Result<MappedValue, MapError> {
    let kind = match fields.text("oracle-kind")? {
        "test" => OracleKind::Test,
        "property" => OracleKind::Property,
        "exhaustive-model" => OracleKind::ExhaustiveModel,
        "structural" => OracleKind::Structural,
        "formal" => OracleKind::Formal,
        "derivation-consistency" => OracleKind::DerivationConsistency,
        "host" => OracleKind::Host,
        other => return Err(MapError::new(format!("invalid-oracle-kind:{other}"))),
    };
    Oracle::new(
        fields.id("id")?,
        kind,
        fields.tool("runner")?,
        fields.payload("selection-schema", "selection")?,
        fields.payload("normalization-schema", "normalization")?,
        fields.digest("configuration-digest")?,
        fields.payload("scope-schema", "scope")?,
        fields.exclusions(),
    )
    .map(MappedValue::Oracle)
    .map_err(|_| MapError::new("invalid-oracle"))
}

/// Maps one extracted record into exactly one repository-independent value.
///
/// # Errors
///
/// Returns a deterministic schema-owned context for every invalid or
/// incomplete record.
pub fn map_record(record: &ChelisRecord) -> Result<MappedValue, MapError> {
    let fields = Fields { record };
    match record.kind.as_str() {
        "authority" => map_authority(&fields),
        "carrier" => map_carrier(&fields),
        "implementation-link" => map_link(&fields),
        "surface" => map_surface(&fields),
        "policy" => map_policy(&fields),
        "debt" => map_debt(&fields),
        "waiver" => map_waiver(&fields),
        "reaffirmation" => map_reaffirmation(&fields),
        "formal-property" => map_formal_property(&fields),
        "derivation" => map_derivation(&fields),
        "oracle" => map_oracle(&fields),
        other => Err(MapError::new(format!("unknown-record-kind:{other}"))),
    }
}
