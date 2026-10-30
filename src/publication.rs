//! Typed, deterministic publication-plan boundary.
#![allow(missing_docs)]

use serde::Deserialize;
use serde_json::Value;
use std::collections::BTreeSet;

pub const PLAN_SCHEMA: &str = "buoy/trace-publication-plan/v1";

/// Maximum accepted raw plan byte length, checked before any JSON
/// allocation.
pub const PLAN_BYTES_BUDGET: usize = 1_048_576;
/// Maximum accepted declared output count.
pub const PLAN_OUTPUT_BUDGET: usize = 256;
/// Maximum accepted declared source-set entry count.
pub const PLAN_SOURCE_SET_BUDGET: usize = 4096;

/// A zero-sized deserialization target that walks one complete JSON document
/// -- every object, at every nesting level -- and fails with an error whose
/// message contains the literal text `duplicate key` the first time the same
/// key appears twice in one object.
///
/// This is the ONE independent pre-pass [`parse_plan`] runs before any typed
/// `RawPlan`/`serde_json::Value` parsing: those later parsers both use
/// `serde_json`'s ordinary map representation, which silently keeps only the
/// LAST occurrence of a duplicate key rather than reject it. An untrusted
/// plan document must never have a conflicting declared field silently
/// resolved that way.
struct DuplicateKeyGuard;

struct DuplicateKeyGuardVisitor;

impl<'de> serde::de::Visitor<'de> for DuplicateKeyGuardVisitor {
    type Value = DuplicateKeyGuard;

    fn expecting(&self, formatter: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        formatter.write_str("any JSON value with no duplicate object keys")
    }

    fn visit_map<A>(self, mut map: A) -> Result<Self::Value, A::Error>
    where
        A: serde::de::MapAccess<'de>,
    {
        let mut seen = BTreeSet::new();
        while let Some(key) = map.next_key::<String>()? {
            if !seen.insert(key.clone()) {
                return Err(serde::de::Error::custom(format!("duplicate key `{key}`")));
            }
            map.next_value::<DuplicateKeyGuard>()?;
        }
        Ok(DuplicateKeyGuard)
    }

    fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error>
    where
        A: serde::de::SeqAccess<'de>,
    {
        while seq.next_element::<DuplicateKeyGuard>()?.is_some() {}
        Ok(DuplicateKeyGuard)
    }

    fn visit_bool<E>(self, _value: bool) -> Result<Self::Value, E> {
        Ok(DuplicateKeyGuard)
    }

    fn visit_i64<E>(self, _value: i64) -> Result<Self::Value, E> {
        Ok(DuplicateKeyGuard)
    }

    fn visit_u64<E>(self, _value: u64) -> Result<Self::Value, E> {
        Ok(DuplicateKeyGuard)
    }

    fn visit_f64<E>(self, _value: f64) -> Result<Self::Value, E> {
        Ok(DuplicateKeyGuard)
    }

    fn visit_str<E>(self, _value: &str) -> Result<Self::Value, E> {
        Ok(DuplicateKeyGuard)
    }

    fn visit_string<E>(self, _value: String) -> Result<Self::Value, E> {
        Ok(DuplicateKeyGuard)
    }

    fn visit_unit<E>(self) -> Result<Self::Value, E> {
        Ok(DuplicateKeyGuard)
    }
}

impl<'de> Deserialize<'de> for DuplicateKeyGuard {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::de::Deserializer<'de>,
    {
        deserializer.deserialize_any(DuplicateKeyGuardVisitor)
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum PublicationError {
    InvalidJson,
    MissingField(String),
    UnknownField(String),
    InvalidField(String),
    OrphanImmutablePage,
    InvalidTargetBase,
    IncompletePublicationKey,
    PlanBytesBudget,
    PlanOutputBudget,
    PlanSourceSetBudget,
    /// More than one typed selector field was present in the plan. This is a
    /// usage fault, not a content fault: the plan names a conflicting request
    /// shape rather than an unresolved value.
    ConflictingSelector,
    /// The requested digest member is unavailable or lacks the declared
    /// assurance class for this operation.
    DigestMemberUnavailable,
    PublicationDigestClass,
}

impl PublicationError {
    /// The stable exit class this diagnostic maps to at the CLI boundary.
    ///
    /// Every content fault (malformed field, unresolved reference, budget
    /// overrun) returns input exit `70`. A conflicting selector shape is a
    /// usage fault and returns exit `64`, matching TPT-002.
    #[must_use]
    pub const fn exit_class(&self) -> u8 {
        match self {
            Self::ConflictingSelector => 64,
            _ => 70,
        }
    }

    /// The stable, versioned `TPT-*` diagnostic code string this fault
    /// reports at the CLI boundary.
    ///
    /// This is the literal string `fixtures/trace-publication/v1/manifest.tsv`
    /// declares for each requirement's negative scenario -- a caller must be
    /// able to observe this exact code in the emitted machine report, not
    /// only an internal Rust enum variant name.
    #[must_use]
    pub fn stable_code(&self) -> &'static str {
        match self {
            Self::InvalidJson => "TPT-PLAN-INVALID-JSON",
            Self::MissingField(_) => "TPT-PLAN-MISSING-FIELD",
            Self::UnknownField(_) => "TPT-PLAN-UNKNOWN-FIELD",
            Self::OrphanImmutablePage => "TPT-PLAN-ORPHAN-IMMUTABLE-PAGE",
            Self::InvalidTargetBase => "TPT-TARGET-BASE",
            Self::IncompletePublicationKey => "TPT-INCOMPLETE-PUBLICATION-KEY",
            Self::PlanBytesBudget => "TPT-PLAN-BYTES-BUDGET",
            Self::PlanOutputBudget => "TPT-PLAN-OUTPUT-BUDGET",
            Self::PlanSourceSetBudget => "TPT-PLAN-SOURCE-SET-BUDGET",
            Self::ConflictingSelector => "TPT-PLAN-CONFLICTING-SELECTOR",
            Self::DigestMemberUnavailable => "TPT-DIGEST-MEMBER-UNAVAILABLE",
            Self::PublicationDigestClass => "TPT-DIGEST-CLASS",
            Self::InvalidField(reason) => match reason.as_str() {
                "path-traversal" => "TPT-PATH-TRAVERSAL",
                "substitution" | "substitution-unavailable" => "TPT-UNKNOWN-SUBSTITUTION",
                "output-escape" => "TPT-OUTPUT-ESCAPE",
                "output-link" => "TPT-OUTPUT-LINK",
                "output-collision" => "TPT-OUTPUT-COLLISION",
                "selected-output" => "TPT-SELECTED-OUTPUT",
                "source-edit-not-selected" => "TPT-SOURCE-EDIT-NOT-SELECTED",
                "target-base-not-page-owned" => "TPT-TARGET-BASE-NOT-PAGE-OWNED",
                "output-path-no-substitution" => "TPT-OUTPUT-PATH-NO-SUBSTITUTION",
                "package-source-identity" => "TPT-PACKAGE-SOURCE-IDENTITY",
                "package-feature" => "TPT-PACKAGE-FEATURE",
                "package-ambient-tool" => "TPT-PACKAGE-AMBIENT-TOOL",
                "completion-evidence" | "completion-evidence-drift" => {
                    "TPT-COMPLETION-EVIDENCE-DRIFT"
                }
                "completion-evidence-placeholder" => "TPT-COMPLETION-EVIDENCE-PLACEHOLDER",
                "claim-boundary" => "TPT-CLAIM-BOUNDARY",
                "external-link relation" => "TPT-EXTERNAL-LINK-RELATION",
                "external-link-missing-page" => "TPT-EXTERNAL-LINK-MISSING-PAGE",
                "external-link-unresolved-page" => "TPT-EXTERNAL-LINK-UNRESOLVED-PAGE",
                "external-link-multiple-owners" => "TPT-EXTERNAL-LINK-MULTIPLE-OWNERS",
                "duplicate output" => "TPT-PLAN-DUPLICATE-OUTPUT",
                "target-traversal" => "TPT-TARGET-TRAVERSAL",
                "report" => "TPT-REPORT-INVALID",
                "package identity" => "TPT-PACKAGE-IDENTITY-INVALID",
                "source-set-entry" => "TPT-SOURCE-SET-ENTRY",
                "source-set-duplicate" => "TPT-SOURCE-SET-DUPLICATE",
                "target-base-missing" => "TPT-TARGET-BASE-MISSING",
                "duplicate-json-key" => "TPT-PLAN-DUPLICATE-KEY",
                "output-id" => "TPT-PLAN-OUTPUT-ID",
                "forbidden-publication-field" => "TPT-FORBIDDEN-PUBLICATION-FIELD",
                _ => "TPT-PLAN-INVALID-FIELD",
            },
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TracePublicationPlan {
    schema: String,
    publication_id: String,
    selector: Selector,
    expected_rows: usize,
    source_set: Vec<String>,
    outputs: Vec<PublicationOutput>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Selector {
    RequirementId(String),
    RelationId(String),
    ImplementationPath(String),
    Record(String),
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PublicationOutput {
    pub id: String,
    pub mount: String,
    pub path: String,
    pub kind: ArtifactKind,
    pub lifecycle: Lifecycle,
    pub relation: Option<String>,
    pub page: Option<String>,
    pub target_base: Option<String>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ArtifactKind {
    Page,
    ExternalLink,
    Evidence,
    SourceEdit,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Lifecycle {
    Immutable,
    Replaceable,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawPlan {
    schema: String,
    publication_id: String,
    selector: RawSelector,
    expected_rows: usize,
    source_set: Vec<String>,
    outputs: Vec<RawOutput>,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawSelector {
    requirement_id: Option<String>,
    relation_id: Option<String>,
    implementation_path: Option<String>,
    record: Option<String>,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawOutput {
    id: String,
    mount: String,
    path: String,
    kind: String,
    lifecycle: String,
    relation: Option<String>,
    page: Option<String>,
    target_base: Option<String>,
}

/// Whether one already-lowercased target-base path segment decodes to a `.`
/// or `..` traversal segment.
///
/// A literal `.`/`..` is the obvious form, but `%2e`, `%2E`, `%2e%2e`,
/// `.%2e`, and `%2e.` all decode to exactly the same traversal and must be
/// rejected identically. Comparing only the literal spelling would accept
/// every percent-encoded variant and let a later join climb outside the
/// declared authority root.
fn decodes_to_dot_segment(segment: &str) -> bool {
    // Decode only the percent-escapes that can produce a `.`; any other
    // escape cannot contribute to a traversal segment, and leaving it
    // undecoded here keeps this check from silently normalizing unrelated
    // bytes.
    let decoded = segment.replace("%2e", ".").replace("%2E", ".");
    decoded == "." || decoded == ".."
}

/// Whether one path segment carries a percent-encoded path separator.
///
/// Round-5 finding H4: `decodes_to_dot_segment` compares a WHOLE segment
/// against `.` and `..`, and segments are split on the LITERAL `/` only. A
/// base containing `%2e%2e%2f%2e%2e` is therefore a single segment that
/// decodes to `../..` -- neither `.` nor `..`, so the traversal check passed
/// it. Any encoded separator lets one apparent segment expand into several
/// real ones after a resolver decodes it, so a normalized base never carries
/// one at all.
fn contains_encoded_separator(segment: &str) -> bool {
    let lowered = segment.to_ascii_lowercase();
    // `%2f` is `/`; `%5c` is `\`, which several real URI parsers also treat
    // as a separator. `%25` is `%` itself: a doubly-encoded escape such as
    // `%252e%252e` decodes to `%2e%2e` on the first pass and to `..` on a
    // second, so a base carrying an encoded percent is not normal either.
    lowered.contains("%2f") || lowered.contains("%5c") || lowered.contains("%25")
}

/// Validates one page target base: an absolute normalized `https` URI with a
/// nonempty authority, a trailing `/`, no query, no fragment, no userinfo, no
/// doubled slash, no literal or percent-encoded traversal segment, and no
/// unresolved substitution braces.
///
/// Normalization is not cosmetic here: a target base is joined with a
/// caller-independent rendered page path to produce the exact immutable
/// target an external link publishes. Every non-normal form below either
/// changes which origin the resulting URI actually resolves against
/// (userinfo), collapses to a different path than it appears to name
/// (doubled slashes, traversal, encoded traversal), or is ambiguous about
/// the authority (empty host, empty port).
///
/// # Errors
///
/// Returns [`PublicationError::InvalidTargetBase`] for any malformed or
/// non-normal base.
fn validate_target_base(base: &str) -> Result<(), PublicationError> {
    if !base.starts_with("https://")
        || !base.ends_with('/')
        || base.contains('?')
        || base.contains('#')
        || base.contains('{')
        || base.contains('}')
        // A control or space byte is never part of a normalized URI.
        || base.chars().any(|c| c.is_control() || c == ' ')
        // A backslash is treated as a path separator by several real URI
        // parsers; a normalized base never contains one.
        || base.contains('\\')
    {
        return Err(PublicationError::InvalidTargetBase);
    }
    let after_scheme = &base["https://".len()..];
    let authority = after_scheme.split('/').next().unwrap_or_default();
    if authority.is_empty() {
        return Err(PublicationError::InvalidTargetBase);
    }
    // Userinfo (`user@host`, `user:password@host`) changes which credentials
    // and, with some parsers, which host the joined URI actually resolves
    // against. A published immutable target must name one unambiguous origin
    // and never embed a credential.
    if authority.contains('@') {
        return Err(PublicationError::InvalidTargetBase);
    }
    // A host with an explicit port must name a real numeric port; `host:`
    // and `host:abc` are both non-normal.
    if let Some((host, port)) = authority.split_once(':') {
        if host.is_empty() || port.is_empty() || !port.bytes().all(|b| b.is_ascii_digit()) {
            return Err(PublicationError::InvalidTargetBase);
        }
    }
    let path_after_authority = &after_scheme[authority.len()..];
    // A doubled slash collapses differently across URI resolvers and never
    // names a normalized path.
    //
    // Round-5 finding H4: this used to strip the trailing `/` before
    // scanning, which made a TERMINAL doubled slash invisible --
    // `https://host/trace//` became `/trace/` and passed. Every valid base
    // ends in exactly one `/`, so scanning the whole path unmodified is both
    // simpler and correct: the only base whose path is a lone `/` is the
    // authority-root form, which contains no `//` either.
    if path_after_authority.contains("//") {
        return Err(PublicationError::InvalidTargetBase);
    }
    for segment in path_after_authority.split('/') {
        if segment.is_empty() {
            // The empty segments produced by the path's own leading and
            // trailing `/` are legitimate; a doubled slash was already
            // rejected above.
            continue;
        }
        if decodes_to_dot_segment(segment) || contains_encoded_separator(segment) {
            return Err(PublicationError::InvalidTargetBase);
        }
    }
    Ok(())
}

/// The complete set of registered version 1 path-template substitutions.
///
/// This is the same closed set [`render_path`] accepts; keeping it as one
/// list here lets plan parsing reject an unregistered substitution (a
/// graph, page, link, page-commit, provider, or unknown key) before any
/// repository operation runs, rather than only the first time an operation
/// actually renders the path.
const REGISTERED_PATH_SUBSTITUTIONS: [&str; 6] = [
    "publication_basis_path",
    "publication_basis",
    "atom_revision_path",
    "atom_revision",
    "record_digest_path",
    "record_digest",
];

/// Validates that every `{...}` substitution in one path template names a
/// registered version 1 substitution, and that every literal `..` traversal
/// segment is absent.
///
/// # Errors
///
/// Returns a deterministic [`PublicationError`] for an unregistered
/// substitution, an unterminated `{`, or a traversal segment.
fn validate_path_template_substitutions(template: &str) -> Result<(), PublicationError> {
    if template.split('/').any(|segment| segment == "..") {
        return Err(PublicationError::InvalidField("path-traversal".into()));
    }
    let mut rest = template;
    while let Some(start) = rest.find('{') {
        let Some(end) = rest[start..].find('}') else {
            return Err(PublicationError::InvalidField("substitution".into()));
        };
        let key = &rest[start + 1..start + end];
        if !REGISTERED_PATH_SUBSTITUTIONS.contains(&key) {
            return Err(PublicationError::InvalidField("substitution".into()));
        }
        rest = &rest[start + end + 1..];
    }
    Ok(())
}

fn parse_output(output: RawOutput) -> Result<PublicationOutput, PublicationError> {
    let kind = match output.kind.as_str() {
        "page" => ArtifactKind::Page,
        "external-link" => ArtifactKind::ExternalLink,
        "evidence" => ArtifactKind::Evidence,
        "source-edit" => ArtifactKind::SourceEdit,
        _ => return Err(PublicationError::InvalidField("kind".into())),
    };
    let lifecycle = match output.lifecycle.as_str() {
        "immutable" => Lifecycle::Immutable,
        "replaceable" => Lifecycle::Replaceable,
        _ => return Err(PublicationError::InvalidField("lifecycle".into())),
    };
    // An output's ID is its only stable handle: `--artifact` selects by it,
    // every emitted `buoy/trace-publication-report/v1` names it in the
    // required `artifact` field, and an external link references its page by
    // it. An empty ID is not a harmless cosmetic defect -- `render_report`
    // rejects an empty `artifact`, so an empty ID silently DISABLES report
    // rendering for that output and turns every one of its diagnostics into
    // a bare exit code with no machine document at all. Reject it at plan
    // parsing, before any repository operation runs.
    if output.id.is_empty() {
        return Err(PublicationError::InvalidField("output-id".into()));
    }
    if output.path.is_empty() || output.mount.is_empty() {
        return Err(PublicationError::InvalidField("output path".into()));
    }
    if matches!(kind, ArtifactKind::ExternalLink) {
        if output.relation.as_deref() != Some("documents") {
            return Err(PublicationError::InvalidField(
                "external-link relation".into(),
            ));
        }
        if output.page.as_deref().is_none_or(str::is_empty) {
            return Err(PublicationError::InvalidField(
                "external-link-missing-page".into(),
            ));
        }
    }
    if matches!(kind, ArtifactKind::Page)
        && matches!(lifecycle, Lifecycle::Immutable)
        && !output.path.contains("{publication_basis_path}")
    {
        return Err(PublicationError::IncompletePublicationKey);
    }
    // TPT-001: "The plan parser compiles each page-path template into
    // literal segments and registered substitutions." Reject every
    // unregistered substitution, and every substitution at all outside a
    // page or evidence output, during PLAN PARSING itself -- not only
    // later, the first time an operation actually renders the path. A
    // graph, page, link, page-commit, provider, or unknown substitution
    // must fail closed before any repository operation runs.
    match kind {
        ArtifactKind::Page | ArtifactKind::Evidence => {
            validate_path_template_substitutions(&output.path)?;
        }
        ArtifactKind::SourceEdit | ArtifactKind::ExternalLink => {
            if output.path.contains('{') || output.path.contains('}') {
                return Err(PublicationError::InvalidField(
                    "output-path-no-substitution".into(),
                ));
            }
        }
    }
    // TPT-001: "A page can declare one normalized absolute target base."
    // Only a page output owns `target_base`; an external-link output only
    // references a declared page by ID and never carries or overrides the
    // complete target itself.
    if output.target_base.is_some() && !matches!(kind, ArtifactKind::Page) {
        return Err(PublicationError::InvalidField(
            "target-base-not-page-owned".into(),
        ));
    }
    if let Some(base) = &output.target_base {
        validate_target_base(base)?;
    }
    Ok(PublicationOutput {
        id: output.id,
        mount: output.mount,
        path: output.path,
        kind,
        lifecycle,
        relation: output.relation,
        page: output.page,
        target_base: output.target_base,
    })
}

/// Parses one untrusted publication plan at the CLI boundary.
///
/// # Errors
///
/// Returns a deterministic [`PublicationError`] for any unknown field,
/// missing field, invalid value, orphan immutable page, or invalid target
/// base.
#[allow(clippy::too_many_lines)]
pub fn parse_plan(bytes: &[u8]) -> Result<TracePublicationPlan, PublicationError> {
    if bytes.len() > PLAN_BYTES_BUDGET {
        return Err(PublicationError::PlanBytesBudget);
    }
    // `serde_json::Value`'s object representation silently collapses a
    // duplicate object key to its last occurrence, at every nesting level.
    // An untrusted plan must never let that silent collapse decide which of
    // two conflicting declared values takes effect: reject any duplicate
    // key, anywhere in the document, before the byte stream ever reaches
    // typed parsing.
    if let Err(error) = serde_json::from_slice::<DuplicateKeyGuard>(bytes) {
        if error.to_string().contains("duplicate key") {
            return Err(PublicationError::InvalidField("duplicate-json-key".into()));
        }
        // Any other failure here is genuinely malformed JSON; the ordinary
        // `Value` parse immediately below reports that case as
        // `InvalidJson`, exactly as it always has.
    }
    let value: Value = serde_json::from_slice(bytes).map_err(|_| PublicationError::InvalidJson)?;
    let object = value.as_object().ok_or(PublicationError::InvalidJson)?;
    for key in object.keys() {
        if !matches!(
            key.as_str(),
            "schema" | "publication_id" | "selector" | "expected_rows" | "source_set" | "outputs"
        ) {
            return Err(PublicationError::UnknownField(key.clone()));
        }
    }
    let raw: RawPlan = serde_json::from_value(value).map_err(|error| {
        let text = error.to_string();
        if text.contains("missing field") {
            let field = text.split('`').nth(1).unwrap_or("unknown").to_owned();
            PublicationError::MissingField(field)
        } else {
            PublicationError::InvalidField(text)
        }
    })?;
    if raw.schema != PLAN_SCHEMA {
        return Err(PublicationError::InvalidField("schema".into()));
    }
    if raw.publication_id.is_empty() || raw.expected_rows == 0 {
        return Err(PublicationError::InvalidField("plan bounds".into()));
    }
    if raw.outputs.len() > PLAN_OUTPUT_BUDGET {
        return Err(PublicationError::PlanOutputBudget);
    }
    if raw.source_set.len() > PLAN_SOURCE_SET_BUDGET {
        return Err(PublicationError::PlanSourceSetBudget);
    }
    // TPT-004's exact source-set policy is a two-directional equality
    // against the resolved selection (see `enforce_source_set_policy`): a
    // genuinely empty declared policy is still a real, enforced "exact
    // equality with the empty set", never a silent bypass. But an empty
    // STRING entry, or a repeated entry, inside a nonempty declared list can
    // never satisfy that equality against any real selection (a resolved
    // logical path is never empty, and a real selector never resolves one
    // path twice), so either one is dead weight that only masks a malformed
    // plan behind a policy that can never legitimately pass. Reject both at
    // parse time, before any repository operation runs.
    let mut source_set_entries = BTreeSet::new();
    for entry in &raw.source_set {
        if entry.is_empty() {
            return Err(PublicationError::InvalidField("source-set-entry".into()));
        }
        if !source_set_entries.insert(entry.clone()) {
            return Err(PublicationError::InvalidField(
                "source-set-duplicate".into(),
            ));
        }
    }
    let selector = selector(raw.selector)?;
    let mut ids = BTreeSet::new();
    let mut paths = BTreeSet::new();
    let mut outputs = Vec::with_capacity(raw.outputs.len());
    for output in raw.outputs {
        if !ids.insert(output.id.clone()) {
            return Err(PublicationError::InvalidField("duplicate output".into()));
        }
        if !paths.insert(output.path.clone()) {
            return Err(PublicationError::InvalidField("output-collision".into()));
        }
        outputs.push(parse_output(output)?);
    }
    // Every external-link output must reference exactly one declared `page`
    // output by ID. A dangling reference and a second link claiming the same
    // page are both rejected before any repository operation runs.
    for link in outputs
        .iter()
        .filter(|output| matches!(output.kind, ArtifactKind::ExternalLink))
    {
        let referenced_page = link.page.as_deref().unwrap_or_default();
        if !outputs
            .iter()
            .any(|output| matches!(output.kind, ArtifactKind::Page) && output.id == referenced_page)
        {
            return Err(PublicationError::InvalidField(
                "external-link-unresolved-page".into(),
            ));
        }
    }
    for page in outputs
        .iter()
        .filter(|output| matches!(output.kind, ArtifactKind::Page))
    {
        let referencing_links = outputs
            .iter()
            .filter(|output| {
                matches!(output.kind, ArtifactKind::ExternalLink)
                    && output.page.as_deref() == Some(page.id.as_str())
            })
            .count();
        if matches!(page.lifecycle, Lifecycle::Immutable) && referencing_links == 0 {
            return Err(PublicationError::OrphanImmutablePage);
        }
        if referencing_links > 1 {
            return Err(PublicationError::InvalidField(
                "external-link-multiple-owners".into(),
            ));
        }
        // TPT-001/TPT-006: a page's `target_base` is what an external link
        // that references it joins into its rendered target. Discovering a
        // missing `target_base` only when `generate` later tries to render
        // that join is a late runtime failure with no structured
        // plan-parse diagnostic -- exactly the gap this closes. Any page
        // that at least one external link references MUST declare
        // `target_base` at PLAN-PARSE time, before any repository
        // operation runs, regardless of that page's own lifecycle.
        if referencing_links > 0 && page.target_base.is_none() {
            return Err(PublicationError::InvalidField("target-base-missing".into()));
        }
    }
    Ok(TracePublicationPlan {
        schema: raw.schema,
        publication_id: raw.publication_id,
        selector,
        expected_rows: raw.expected_rows,
        source_set: raw.source_set,
        outputs,
    })
}

fn selector(raw: RawSelector) -> Result<Selector, PublicationError> {
    let values = [
        raw.requirement_id.map(Selector::RequirementId),
        raw.relation_id.map(Selector::RelationId),
        raw.implementation_path.map(Selector::ImplementationPath),
        raw.record.map(Selector::Record),
    ];
    let mut selected = values.into_iter().flatten();
    let first = selected
        .next()
        .ok_or_else(|| PublicationError::MissingField("selector".into()))?;
    if selected.next().is_some() {
        return Err(PublicationError::ConflictingSelector);
    }
    Ok(first)
}

impl TracePublicationPlan {
    #[must_use]
    pub fn schema(&self) -> &str {
        &self.schema
    }
    #[must_use]
    pub fn publication_id(&self) -> &str {
        &self.publication_id
    }
    #[must_use]
    pub fn selector(&self) -> &Selector {
        &self.selector
    }
    #[must_use]
    pub fn expected_rows(&self) -> usize {
        self.expected_rows
    }
    #[must_use]
    pub fn source_set(&self) -> &[String] {
        &self.source_set
    }
    #[must_use]
    pub fn outputs(&self) -> &[PublicationOutput] {
        &self.outputs
    }
    #[must_use]
    pub fn immutable_page_count(&self) -> usize {
        let mut count = 0;
        for output in &self.outputs {
            if matches!(output.kind, ArtifactKind::Page)
                && matches!(output.lifecycle, Lifecycle::Immutable)
            {
                count += 1;
            }
        }
        count
    }
}

pub const REPORT_SCHEMA: &str = "buoy/trace-publication-report/v1";

/// One publication `check` or `generate` operation report.
///
/// Every optional field is absent until a caller explicitly supplies it.
/// This makes the type itself express TPT-011's stage rule: an unavailable
/// field never renders a placeholder or fabricated identity, only a truly
/// absent JSON key.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PublicationReport {
    operation: String,
    artifact: String,
    plan_id: Option<String>,
    basis: Option<String>,
    graph: Option<String>,
    prior_state: Option<String>,
    prior_digest: Option<String>,
    candidate_digest: Option<String>,
    outcome: Option<String>,
    diagnostics: Vec<(String, String)>,
    scope_claim: Option<String>,
    parity: Option<String>,
}

impl PublicationReport {
    #[must_use]
    pub fn new(operation: &str, artifact: &str) -> Self {
        Self {
            operation: operation.into(),
            artifact: artifact.into(),
            plan_id: None,
            basis: None,
            graph: None,
            prior_state: None,
            prior_digest: None,
            candidate_digest: None,
            outcome: None,
            diagnostics: Vec::new(),
            scope_claim: None,
            parity: None,
        }
    }
    #[must_use]
    pub fn with_diagnostic(mut self, code: &str, message: &str) -> Self {
        self.diagnostics.push((code.into(), message.into()));
        self
    }
    #[must_use]
    pub fn with_scope_claim(mut self, claim: &str) -> Self {
        self.scope_claim = Some(claim.into());
        self
    }
    /// Attaches the plan's declared publication ID.
    #[must_use]
    pub fn with_plan_id(mut self, plan_id: &str) -> Self {
        self.plan_id = Some(plan_id.into());
        self
    }
    /// Attaches the publication-basis digest this operation computed.
    #[must_use]
    pub fn with_basis(mut self, basis: &str) -> Self {
        self.basis = Some(basis.into());
        self
    }
    /// Attaches the current source-graph digest this operation observed.
    #[must_use]
    pub fn with_graph(mut self, graph: &str) -> Self {
        self.graph = Some(graph.into());
        self
    }
    /// Attaches the prior artifact state (`"absent"`, `"equal"`, or
    /// `"unequal"`) and its exact digest, when one existed.
    #[must_use]
    pub fn with_prior(mut self, state: &str, digest: Option<&str>) -> Self {
        self.prior_state = Some(state.into());
        self.prior_digest = digest.map(str::to_owned);
        self
    }
    /// Attaches the exact candidate digest this operation rendered.
    #[must_use]
    pub fn with_candidate_digest(mut self, digest: &str) -> Self {
        self.candidate_digest = Some(digest.into());
        self
    }
    /// Attaches the deterministic, fixed-label parity summary between the
    /// saved bytes and this operation's candidate.
    ///
    /// # This operation cannot fail, and that is the contract
    ///
    /// Round-5 finding H1: this used to return a `Result`, and the caller
    /// turned an `Err` into a blocked outcome with
    /// `TPT-FORBIDDEN-PUBLICATION-FIELD`. Because the rendered summary
    /// quoted document content, ordinary page bytes could produce that
    /// `Err` -- so arbitrary existing page content could change the machine
    /// exit class and MASK a real immutable conflict.
    ///
    /// [`render_parity_diff`] now emits only renderer-owned metadata, so
    /// there is no content-dependent failure mode left to report. The
    /// forbidden-field contract is still enforced here, but it is enforced
    /// against the RENDERER's own metadata, and a renderer regression
    /// degrades to a fixed placeholder instead of altering the outcome: a
    /// parity detail is operator convenience, never the machine contract.
    #[must_use]
    pub fn with_parity(mut self, existing: Option<&[u8]>, candidate: &[u8]) -> Self {
        self.parity = Some(validated_parity(render_parity_diff(existing, candidate)));
        self
    }
    /// The deterministic parity summary, when attached.
    #[must_use]
    pub fn parity(&self) -> Option<&str> {
        self.parity.as_deref()
    }
    /// Attaches the exact local outcome (`"no-op"`, `"created"`,
    /// `"replaced"`, `"conflict"`, or `"blocked"`) this operation completed.
    ///
    /// This identifies only the local replacement this operation performed;
    /// it never claims a repository commit or provider publication.
    #[must_use]
    pub fn with_outcome(mut self, outcome: &str) -> Self {
        self.outcome = Some(outcome.into());
        self
    }
    #[must_use]
    pub fn operation(&self) -> &str {
        &self.operation
    }
    /// The declared artifact this report names.
    #[must_use]
    pub fn artifact(&self) -> &str {
        &self.artifact
    }
    /// The plan's declared publication ID, when attached.
    #[must_use]
    pub fn plan_id(&self) -> Option<&str> {
        self.plan_id.as_deref()
    }
    /// The computed publication-basis digest, when attached.
    #[must_use]
    pub fn basis(&self) -> Option<&str> {
        self.basis.as_deref()
    }
    /// The current source-graph digest, when attached.
    #[must_use]
    pub fn graph(&self) -> Option<&str> {
        self.graph.as_deref()
    }
    /// The prior artifact state (`"absent"`, `"equal"`, or `"unequal"`),
    /// when attached.
    #[must_use]
    pub fn prior_state(&self) -> Option<&str> {
        self.prior_state.as_deref()
    }
    /// The prior artifact's exact digest, when one existed.
    #[must_use]
    pub fn prior_digest(&self) -> Option<&str> {
        self.prior_digest.as_deref()
    }
    /// The exact candidate digest this operation rendered, when attached.
    #[must_use]
    pub fn candidate_digest(&self) -> Option<&str> {
        self.candidate_digest.as_deref()
    }
    /// The exact local outcome this operation completed, when attached.
    #[must_use]
    pub fn outcome(&self) -> Option<&str> {
        self.outcome.as_deref()
    }
    /// Every attached `(code, message)` diagnostic pair, in insertion
    /// order (the same order [`render_report`] sorts before rendering).
    #[must_use]
    pub fn diagnostics(&self) -> &[(String, String)] {
        &self.diagnostics
    }
}

fn validated_parity(rendered: String) -> String {
    match validate_publication_diagnostic_text(&rendered) {
        Ok(()) => rendered,
        // Unreachable for production renderer output. The separate helper
        // keeps this future-regression fallback directly testable.
        Err(_) => String::from("parity: unavailable (renderer metadata rejected)"),
    }
}

/// Renders one publication operation report to its canonical machine bytes.
///
/// # Errors
///
/// Returns a deterministic [`PublicationError`] when the report claims a
/// cross-repository scope, or when a required field is empty.
pub fn render_report(report: &PublicationReport) -> Result<Vec<u8>, PublicationError> {
    if report.operation.is_empty() || report.artifact.is_empty() || report.scope_claim.is_some() {
        return Err(PublicationError::InvalidField("report".into()));
    }
    // TPT-009: the machine report IS the automation contract, so it must
    // never carry a timestamp, temporary name, physical root, process ID,
    // locale value, or raw diff header in any diagnostic code or message.
    // Validating here, at the one canonical rendering boundary, means no
    // caller can construct a report that renders forbidden content.
    for (code, message) in &report.diagnostics {
        validate_publication_diagnostic_text(code)?;
        validate_publication_diagnostic_text(message)?;
    }
    let mut diagnostics = report.diagnostics.clone();
    diagnostics.sort();
    let diagnostics = diagnostics
        .into_iter()
        .map(|(code, message)| serde_json::json!({"code": code, "message": message}))
        .collect::<Vec<_>>();
    let mut object = serde_json::Map::new();
    object.insert(
        "artifact".into(),
        serde_json::Value::String(report.artifact.clone()),
    );
    object.insert("diagnostics".into(), serde_json::Value::Array(diagnostics));
    object.insert(
        "operation".into(),
        serde_json::Value::String(report.operation.clone()),
    );
    object.insert(
        "schema".into(),
        serde_json::Value::String(REPORT_SCHEMA.into()),
    );
    object.insert(
        "stage".into(),
        serde_json::Value::String(report.operation.clone()),
    );
    if let Some(plan_id) = &report.plan_id {
        object.insert("plan_id".into(), serde_json::Value::String(plan_id.clone()));
    }
    if let Some(basis) = &report.basis {
        object.insert("basis".into(), serde_json::Value::String(basis.clone()));
    }
    if let Some(graph) = &report.graph {
        object.insert("graph".into(), serde_json::Value::String(graph.clone()));
    }
    if let Some(prior_state) = &report.prior_state {
        object.insert(
            "prior_state".into(),
            serde_json::Value::String(prior_state.clone()),
        );
    }
    if let Some(prior_digest) = &report.prior_digest {
        object.insert(
            "prior_digest".into(),
            serde_json::Value::String(prior_digest.clone()),
        );
    }
    if let Some(candidate_digest) = &report.candidate_digest {
        object.insert(
            "candidate_digest".into(),
            serde_json::Value::String(candidate_digest.clone()),
        );
    }
    if let Some(outcome) = &report.outcome {
        object.insert("outcome".into(), serde_json::Value::String(outcome.clone()));
    }
    serde_json::to_vec(&serde_json::Value::Object(object))
        .map_err(|_| PublicationError::InvalidJson)
}

/// Renders one publication operation report as deterministic human text,
/// derived entirely from the same typed value [`render_report`] encodes.
///
/// TPT-009 keeps the machine document as the automation contract, so this
/// renderer never introduces a fact the typed report does not already carry:
/// every line below reads one field of `report`. The optional parity section
/// is the fixed-label [`render_parity_diff`] summary the operation attached,
/// which is why a human reader sees WHICH lines differ while the machine
/// document stays a stable, minimal contract.
///
/// # Errors
///
/// Returns the same [`PublicationError`] [`render_report`] returns for a
/// report that claims a cross-repository scope, omits a required field, or
/// carries forbidden diagnostic content.
pub fn render_report_human(report: &PublicationReport) -> Result<String, PublicationError> {
    // Validate through exactly the same gate the machine encoder uses, so
    // human output can never be emitted for a report the machine encoder
    // would reject.
    render_report(report)?;
    if let Some(parity) = &report.parity {
        validate_publication_diagnostic_text(parity)?;
    }
    let mut lines = vec![
        format!("schema: {REPORT_SCHEMA}"),
        format!("operation: {}", report.operation),
        format!("artifact: {}", report.artifact),
    ];
    for (label, value) in [
        ("plan", report.plan_id.as_deref()),
        ("basis", report.basis.as_deref()),
        ("graph", report.graph.as_deref()),
        ("prior state", report.prior_state.as_deref()),
        ("prior digest", report.prior_digest.as_deref()),
        ("candidate digest", report.candidate_digest.as_deref()),
        ("outcome", report.outcome.as_deref()),
    ] {
        if let Some(value) = value {
            lines.push(format!("{label}: {value}"));
        }
    }
    let mut diagnostics = report.diagnostics.clone();
    diagnostics.sort();
    for (code, message) in diagnostics {
        lines.push(format!("diagnostic: {code} {message}"));
    }
    if let Some(parity) = &report.parity {
        lines.push(String::from("parity:"));
        for line in parity.lines() {
            lines.push(format!("  {line}"));
        }
    }
    Ok(lines.join("\n") + "\n")
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PublicationEvidence {
    publication_id: String,
    source_digest: Option<String>,
    selector: Option<String>,
    basis: Option<String>,
    graph: Option<String>,
    rows: Vec<TraceRow>,
    link_target: Option<String>,
    page_digest: Option<String>,
    diagnostics: Vec<(String, String)>,
    status: EvidenceStatus,
}

/// Whether one rendered evidence document reports a complete historical
/// summary or a blocked operation that could not produce one.
///
/// `evidence` is the one publication operation whose whole product IS its
/// machine document, so a failed `evidence` run must still emit that
/// document -- carrying the exact blocking diagnostic -- rather than exit
/// with empty output and leave the caller nothing machine-readable to act
/// on.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum EvidenceStatus {
    /// A complete historical evidence summary.
    #[default]
    Historical,
    /// The operation could not produce a complete summary; at least one
    /// blocking diagnostic is attached.
    Blocked,
}

impl EvidenceStatus {
    /// The exact `status` string this state renders.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Historical => "historical",
            Self::Blocked => "blocked",
        }
    }
}

impl PublicationEvidence {
    pub const SCHEMA: &'static str = "buoy/trace-publication-evidence/v1";
    #[must_use]
    pub fn new(publication_id: &str) -> Self {
        Self {
            publication_id: publication_id.into(),
            source_digest: None,
            selector: None,
            basis: None,
            graph: None,
            rows: Vec::new(),
            link_target: None,
            page_digest: None,
            diagnostics: Vec::new(),
            status: EvidenceStatus::Historical,
        }
    }
    /// Attaches one stable blocking diagnostic and marks this evidence
    /// document `blocked`.
    ///
    /// A blocked evidence document still renders every identity the
    /// operation did resolve, so a caller can see exactly how far the
    /// operation got; it simply never claims to be a complete historical
    /// summary.
    #[must_use]
    pub fn with_diagnostic(mut self, code: &str, message: &str) -> Self {
        self.diagnostics.push((code.into(), message.into()));
        self.status = EvidenceStatus::Blocked;
        self
    }
    /// Every attached `(code, message)` diagnostic pair, in insertion order
    /// (the same order [`render_evidence`] sorts before rendering).
    #[must_use]
    pub fn diagnostics(&self) -> &[(String, String)] {
        &self.diagnostics
    }
    /// Whether this document reports a complete summary or a blocked
    /// operation.
    #[must_use]
    pub const fn status(&self) -> EvidenceStatus {
        self.status
    }
    /// Attaches the retained-snapshot identity this evidence derives from.
    ///
    /// This is the one identity the evidence renderer can supply from a
    /// real retained snapshot without re-reading the repository. It is
    /// never fabricated: callers pass a digest already computed over
    /// canonically ordered retained bytes.
    #[must_use]
    pub fn with_source_digest(mut self, digest: &str) -> Self {
        self.source_digest = Some(digest.into());
        self
    }
    /// Attaches the plan's typed selector identity (`"<kind> <value>"`).
    #[must_use]
    pub fn with_selector(mut self, selector: &Selector) -> Self {
        self.selector = Some(match selector {
            Selector::RequirementId(value) => format!("requirement-id {value}"),
            Selector::RelationId(value) => format!("relation-id {value}"),
            Selector::ImplementationPath(value) => format!("implementation-path {value}"),
            Selector::Record(value) => format!("record {value}"),
        });
        self
    }
    /// Attaches the publication-basis digest this evidence was rendered under.
    #[must_use]
    pub fn with_basis(mut self, basis: &str) -> Self {
        self.basis = Some(basis.into());
        self
    }
    /// Attaches the current source-graph digest.
    #[must_use]
    pub fn with_graph(mut self, graph: &str) -> Self {
        self.graph = Some(graph.into());
        self
    }
    /// Attaches the exact current trace rows this evidence summarizes.
    ///
    /// Evidence carries only the same row-local facts `buoy trace` already
    /// exposes (relation, record, requirement, atom revision, both logical
    /// paths, state); it never copies an immutable record body.
    #[must_use]
    pub fn with_rows(mut self, rows: &[TraceRow]) -> Self {
        self.rows = rows.to_vec();
        self
    }
    /// Attaches the declared external-link target, when the plan declares one.
    #[must_use]
    pub fn with_link_target(mut self, target: &str) -> Self {
        self.link_target = Some(target.into());
        self
    }
    /// Attaches the declared page's exact content digest, when the plan
    /// declares one.
    #[must_use]
    pub fn with_page_digest(mut self, digest: &str) -> Self {
        self.page_digest = Some(digest.into());
        self
    }
    #[must_use]
    pub fn publication_id(&self) -> &str {
        &self.publication_id
    }
    /// The retained-snapshot source digest, when attached.
    #[must_use]
    pub fn source_digest(&self) -> Option<&str> {
        self.source_digest.as_deref()
    }
    /// The plan's typed selector identity text, when attached.
    #[must_use]
    pub fn selector_text(&self) -> Option<&str> {
        self.selector.as_deref()
    }
    /// The computed publication-basis digest, when attached.
    #[must_use]
    pub fn basis(&self) -> Option<&str> {
        self.basis.as_deref()
    }
    /// The current source-graph digest, when attached.
    #[must_use]
    pub fn graph(&self) -> Option<&str> {
        self.graph.as_deref()
    }
    /// Every attached current trace row.
    #[must_use]
    pub fn rows(&self) -> &[TraceRow] {
        &self.rows
    }
    /// The declared external-link target, when attached.
    #[must_use]
    pub fn link_target(&self) -> Option<&str> {
        self.link_target.as_deref()
    }
    /// The declared page's exact content digest, when attached.
    #[must_use]
    pub fn page_digest(&self) -> Option<&str> {
        self.page_digest.as_deref()
    }
    #[must_use]
    pub fn render_markdown(&self) -> String {
        if self.publication_id.is_empty() {
            return String::new();
        }
        let mut lines = vec![format!("Publication ID: {}", self.publication_id)];
        lines.push(format!("Status: {}", self.status.as_str()));
        let mut diagnostics = self.diagnostics.clone();
        diagnostics.sort();
        for (code, message) in diagnostics {
            lines.push(format!("Blocker: {code} {message}"));
        }
        if let Some(selector) = &self.selector {
            lines.push(format!("Selector: {selector}"));
        }
        if let Some(basis) = &self.basis {
            lines.push(format!("Publication basis: {basis}"));
        }
        if let Some(graph) = &self.graph {
            lines.push(format!("Source graph: {graph}"));
        }
        if let Some(digest) = &self.source_digest {
            lines.push(format!("Source digest: {digest}"));
        }
        if let Some(target) = &self.link_target {
            lines.push(format!("External-link target: {target}"));
        }
        if let Some(digest) = &self.page_digest {
            lines.push(format!("Page digest: {digest}"));
        }
        for row in &self.rows {
            lines.push(format!(
                "Row: relation={} record={} requirement={} atom_revision={} requirement_path={} implementation_path={} state={} freshness={:?}",
                row.relation.as_deref().unwrap_or("unretained"),
                row.record.as_deref().unwrap_or("unretained"),
                row.requirement_id.as_deref().unwrap_or("unretained"),
                row.atom_revision.as_deref().unwrap_or("unretained"),
                row.requirement_path.as_deref().unwrap_or("unretained"),
                row.implementation_path.as_deref().unwrap_or("unretained"),
                row.state.as_deref().unwrap_or("unretained"),
                row.freshness,
            ));
        }
        format!(
            "<!-- BEGIN buoy-trace-publication-evidence-v1 -->\n{}\n\nBuoy remains the current-state authority. This historical data supplies navigation and impact only. It makes no assurance claim.\n<!-- END buoy-trace-publication-evidence-v1 -->",
            lines.join("\n")
        )
    }
}

#[must_use]
pub fn render_evidence(evidence: &PublicationEvidence) -> Vec<u8> {
    if evidence.publication_id.is_empty() {
        return Vec::new();
    }
    let rows: Vec<serde_json::Value> = evidence
        .rows
        .iter()
        .map(|row| {
            serde_json::json!({
                "relation": row.relation,
                "record": row.record,
                "requirement_id": row.requirement_id,
                "atom_revision": row.atom_revision,
                "requirement_path": row.requirement_path,
                "implementation_path": row.implementation_path,
                "state": row.state,
                "freshness": row.freshness.map(|freshness| serde_json::json!({
                    "record": freshness.record,
                    "attachment": freshness.attachment,
                    "implementation": freshness.implementation,
                    "requirement": freshness.requirement,
                    "authority": freshness.authority,
                })),
            })
        })
        .collect();
    let mut diagnostics = evidence.diagnostics.clone();
    diagnostics.sort();
    let diagnostics: Vec<serde_json::Value> = diagnostics
        .into_iter()
        .map(|(code, message)| serde_json::json!({"code": code, "message": message}))
        .collect();
    let mut object = serde_json::Map::new();
    object.insert("diagnostics".into(), serde_json::Value::Array(diagnostics));
    object.insert(
        "publication_id".into(),
        serde_json::Value::String(evidence.publication_id.clone()),
    );
    object.insert(
        "schema".into(),
        serde_json::Value::String(PublicationEvidence::SCHEMA.into()),
    );
    object.insert(
        "status".into(),
        serde_json::Value::String(evidence.status.as_str().into()),
    );
    if let Some(digest) = &evidence.source_digest {
        object.insert(
            "source_digest".into(),
            serde_json::Value::String(digest.clone()),
        );
    }
    if let Some(selector) = &evidence.selector {
        object.insert(
            "selector".into(),
            serde_json::Value::String(selector.clone()),
        );
    }
    if let Some(basis) = &evidence.basis {
        object.insert("basis".into(), serde_json::Value::String(basis.clone()));
    }
    if let Some(graph) = &evidence.graph {
        object.insert("graph".into(), serde_json::Value::String(graph.clone()));
    }
    if !rows.is_empty() {
        object.insert("rows".into(), serde_json::Value::Array(rows));
    }
    if let Some(target) = &evidence.link_target {
        object.insert(
            "link_target".into(),
            serde_json::Value::String(target.clone()),
        );
    }
    if let Some(digest) = &evidence.page_digest {
        object.insert(
            "page_digest".into(),
            serde_json::Value::String(digest.clone()),
        );
    }
    serde_json::to_vec(&serde_json::Value::Object(object)).unwrap_or_default()
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PackageIdentity {
    version: String,
    source_revision: String,
    toolchain: String,
    features: Vec<String>,
}
impl PackageIdentity {
    pub const SCHEMA: &'static str = "buoy/package-identity/v1";
    #[must_use]
    pub fn new(
        version: &str,
        source_revision: &str,
        toolchain: &str,
        mut features: Vec<&str>,
    ) -> Self {
        features.sort_unstable();
        Self {
            version: version.into(),
            source_revision: source_revision.into(),
            toolchain: toolchain.into(),
            features: features.into_iter().map(str::to_owned).collect(),
        }
    }
    #[must_use]
    pub fn source_revision(&self) -> &str {
        &self.source_revision
    }
    /// Renders one package-identity value to its canonical machine bytes.
    ///
    /// # Errors
    ///
    /// Returns a deterministic [`PublicationError`] when any required field
    /// is empty or the declared feature set is incomplete.
    pub fn render(&self) -> Result<Vec<u8>, PublicationError> {
        if self.version.is_empty()
            || self.source_revision.is_empty()
            || self.toolchain.is_empty()
            || self.features.is_empty()
            || self.features.iter().any(String::is_empty)
        {
            return Err(PublicationError::InvalidField("package identity".into()));
        }
        let value = serde_json::json!({"features": self.features, "operational": true, "schema": Self::SCHEMA, "source_revision": self.source_revision, "supported_members": ["blake3-256", "sha-256", "xxh3-128"], "toolchain": self.toolchain, "version": self.version});
        serde_json::to_vec(&value).map_err(|_| PublicationError::InvalidJson)
    }
}

/// The three distinct typed identities a registered path template can
/// substitute.
///
/// Each identity is independent: `basis` is the computed publication-basis
/// digest, `atom_revision` is the selected trace's own atom-revision digest,
/// and `record_digest` is the selected trace's own binding-record digest.
/// They MUST NOT collapse to the same value except by real coincidence of
/// the underlying bytes -- a template using `{atom_revision_path}` renders
/// the atom revision, never the basis, and likewise for the record digest.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct PathIdentities<'a> {
    pub basis: &'a str,
    /// The selected trace's unambiguous atom-revision identity, when one
    /// exists. `None` when the selected trace carries no single unambiguous
    /// value (for example, multiple rows disagree).
    pub atom_revision: Option<&'a str>,
    /// The selected trace's unambiguous binding-record digest, when one
    /// exists.
    pub record_digest: Option<&'a str>,
}

/// Render one registered publication path substitution.
///
/// Version 1 registers `publication_basis_path`, `publication_basis`,
/// `atom_revision_path`, `atom_revision`, `record_digest_path`, and
/// `record_digest`. Every other substitution, and every literal `..`
/// traversal segment, fails closed.
///
/// # Errors
///
/// Returns a deterministic [`PublicationError`] for an unregistered
/// substitution, a traversal segment, or a registered substitution whose
/// declared typed identity is unavailable for this selected trace.
pub fn render_path(
    template: &str,
    identities: &PathIdentities<'_>,
) -> Result<String, PublicationError> {
    if template.split('/').any(|segment| segment == "..") {
        return Err(PublicationError::InvalidField("path-traversal".into()));
    }
    let mut output = String::new();
    let mut rest = template;
    while let Some(start) = rest.find('{') {
        output.push_str(&rest[..start]);
        let Some(end) = rest[start..].find('}') else {
            return Err(PublicationError::InvalidField("substitution".into()));
        };
        let key = &rest[start + 1..start + end];
        let value = match key {
            "publication_basis_path" | "publication_basis" => Some(identities.basis),
            "atom_revision_path" | "atom_revision" => identities.atom_revision,
            "record_digest_path" | "record_digest" => identities.record_digest,
            _ => return Err(PublicationError::InvalidField("substitution".into())),
        };
        let value = value
            .ok_or_else(|| PublicationError::InvalidField("substitution-unavailable".into()))?;
        let (algorithm, payload) = value
            .split_once(':')
            .ok_or_else(|| PublicationError::InvalidField("substitution".into()))?;
        if key.ends_with("_path") {
            output.push_str(algorithm);
            output.push('/');
            output.push_str(payload);
        } else {
            output.push_str(algorithm);
            output.push(':');
            output.push_str(payload);
        }
        rest = &rest[start + end + 1..];
    }
    output.push_str(rest);
    Ok(output)
}

/// The five row-local freshness facets `buoy trace` already exposes
/// (`buoy/trace-report/v1`'s `freshness` object), carried unchanged into
/// publication row conformance.
///
/// This is a fragmentation guard, not a redundant staleness check: `state`
/// already distinguishes `"current"` from `"stale"`, but a second row could
/// otherwise supply an accompanying freshness fact for a first row that
/// omits one. Requiring this struct to be present on the same row that
/// carries every other row-local fact closes that gap.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
#[allow(clippy::struct_excessive_bools)]
pub struct RowFreshness {
    pub record: bool,
    pub attachment: bool,
    pub implementation: bool,
    pub requirement: bool,
    pub authority: bool,
}

/// One typed trace-report row used by publication row conformance.
///
/// Every field is optional so fragmentation fixtures can represent rows that
/// supply only part of an expected relation.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct TraceRow {
    pub relation: Option<String>,
    pub record: Option<String>,
    pub requirement_id: Option<String>,
    pub atom_revision: Option<String>,
    pub requirement_node: Option<String>,
    pub implementation_node: Option<String>,
    pub requirement_path: Option<String>,
    pub implementation_path: Option<String>,
    pub state: Option<String>,
    pub freshness: Option<RowFreshness>,
    pub diagnostics: Vec<String>,
}

impl TraceRow {
    /// Whether this one row independently carries every row-local fact
    /// TPT-003 requires: relation, record, requirement, atom revision, both
    /// nodes, both logical paths, and its own freshness facets — with no
    /// diagnostic attached. A second row can never complete a first row's
    /// missing value for any of these facts.
    fn is_complete(&self) -> bool {
        self.relation.is_some()
            && self.record.is_some()
            && self.requirement_id.is_some()
            && self.atom_revision.is_some()
            && self.requirement_node.is_some()
            && self.implementation_node.is_some()
            && self.requirement_path.is_some()
            && self.implementation_path.is_some()
            && self.freshness.is_some()
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ConformanceError {
    RowCount,
    RowFragmentation,
    Stale,
    /// A structurally complete, current row still carries its own
    /// unresolved diagnostic. In production this state is already
    /// unreachable (the CLI boundary rejects any nonempty report-level
    /// diagnostic set before row conformance ever runs), but row
    /// conformance itself must not silently accept it: TPT-003 requires
    /// diagnostics to be enforced on the same row as every other fact.
    RowDiagnostic,
}

impl ConformanceError {
    /// The stable, versioned `TPT-*` diagnostic code string this row-level
    /// drift fault reports at the CLI boundary.
    ///
    /// This comment previously claimed `Stale` reported the exact code
    /// `fixtures/trace-publication/v1/manifest.tsv` declares for TPT-003's
    /// negative scenario. It did not: that scenario
    /// (`negative-expected-fragments-occur-on-different-rows`) really
    /// observes `TPT-ROW-COUNT`, which the manifest now records because its
    /// harness executes the command and reads the code out of real output
    /// rather than out of this file.
    #[must_use]
    pub const fn stable_code(self) -> &'static str {
        match self {
            Self::RowCount => "TPT-ROW-COUNT",
            Self::RowFragmentation => "TPT-ROW-FRAGMENTATION",
            Self::Stale => "TPT-ROW-CONFORMANCE",
            Self::RowDiagnostic => "TPT-ROW-DIAGNOSTIC",
        }
    }
}

/// Enforces exact row cardinality, then row-local completeness, then
/// freshness, then row-local diagnostics.
///
/// A second row can never supply a value absent from the first: each accepted
/// row must independently carry every field the plan requires (relation,
/// record, requirement, atom revision, both nodes, both logical paths, and
/// its own freshness facets). A row's own diagnostics are enforced last so a
/// row that is both stale and separately diagnosed is reported as `Stale`,
/// its more specific fact.
///
/// # Errors
///
/// Returns a deterministic [`ConformanceError`] for a row-count mismatch, a
/// fragmented match across rows, a selected stale row, or a structurally
/// complete current row that still carries its own diagnostic.
pub fn row_conformance(rows: &[TraceRow], expected_rows: usize) -> Result<(), ConformanceError> {
    if rows.len() != expected_rows {
        return Err(ConformanceError::RowCount);
    }
    // Every accepted row must independently carry every required field; a
    // second row can never complete a first row's missing values.
    if !rows.iter().all(TraceRow::is_complete) {
        return Err(ConformanceError::RowFragmentation);
    }
    if rows
        .iter()
        .any(|row| row.state.as_deref() != Some("current"))
    {
        return Err(ConformanceError::Stale);
    }
    if rows.iter().any(|row| !row.diagnostics.is_empty()) {
        return Err(ConformanceError::RowDiagnostic);
    }
    Ok(())
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum OutputClass {
    Derived,
    SourceEdit,
}

/// Classifies one declared output path against its plan's selected source set.
///
/// A derived output must remain outside every selected source. A source-edit
/// output must select exactly one regular selected source path. Link-like and
/// escaping paths fail closed before any temporary artifact exists.
///
/// # Errors
///
/// Returns a deterministic [`PublicationError`] for a root-escaping,
/// link-like, unselected source-edit, or selected derived-output path.
pub fn classify_output(
    path: &str,
    kind: &str,
    source_set: &[String],
) -> Result<OutputClass, PublicationError> {
    if path.is_empty() || path.starts_with('/') {
        return Err(PublicationError::InvalidField("output-escape".into()));
    }
    if path
        .split('/')
        .any(|segment| segment == ".." || segment == "." || segment.is_empty())
    {
        return Err(PublicationError::InvalidField("output-escape".into()));
    }
    if path.ends_with("/link") || path.contains("/link/") {
        return Err(PublicationError::InvalidField("output-link".into()));
    }
    let selected = source_set
        .iter()
        .any(|selected| paths_alias(selected, path));
    match kind {
        "source-edit" if selected => Ok(OutputClass::SourceEdit),
        "source-edit" => Err(PublicationError::InvalidField(
            "source-edit-not-selected".into(),
        )),
        _ if selected => Err(PublicationError::InvalidField("selected-output".into())),
        _ => Ok(OutputClass::Derived),
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SourceGuardError {
    Absent,
    Stale,
}

/// Requires one declared source-edit path to equal its candidate bytes in
/// the retained snapshot before any derived-artifact operation proceeds.
///
/// `snapshot` pairs each selected logical path with its exact retained
/// bytes. A missing path or byte mismatch both return static-blocker facts;
/// callers translate either into exit `2` before rendering.
///
/// # Errors
///
/// Returns [`SourceGuardError::Absent`] when the declared path is not in the
/// retained snapshot, and [`SourceGuardError::Stale`] when its bytes differ
/// from the candidate.
pub fn require_source_edit_conforms(
    snapshot: &[(String, Vec<u8>)],
    declared_path: &str,
    candidate: &[u8],
) -> Result<(), SourceGuardError> {
    let Some((_, bytes)) = snapshot.iter().find(|(path, _)| path == declared_path) else {
        return Err(SourceGuardError::Absent);
    };
    if bytes.as_slice() == candidate {
        Ok(())
    } else {
        Err(SourceGuardError::Stale)
    }
}

/// Re-derives one guard decision for every selected path immediately before
/// output replacement, using a freshly reopened snapshot.
///
/// A changed, removed, replaced, or newly appeared selected path all abort
/// the write. The candidate keeps its retained-snapshot identity but never
/// reaches the output.
///
/// # Errors
///
/// Returns [`SourceGuardError::Stale`] for any drift between the retained
/// and the reopened selected sets, including a path that no longer exists
/// or a path that appeared after the retained snapshot was captured.
pub fn reverify_selected_paths(
    retained: &[(String, Vec<u8>)],
    reopened: &[(String, Vec<u8>)],
) -> Result<(), SourceGuardError> {
    if retained.len() != reopened.len() {
        return Err(SourceGuardError::Stale);
    }
    for (path, bytes) in retained {
        match reopened
            .iter()
            .find(|(candidate_path, _)| candidate_path == path)
        {
            Some((_, candidate_bytes)) if candidate_bytes == bytes => {}
            _ => return Err(SourceGuardError::Stale),
        }
    }
    Ok(())
}

/// Compares one plan's exact source-set policy against one selector's
/// resolved logical paths.
///
/// An exact policy detects a recursive selector that grew beyond its
/// declared one-file (or explicit multi-file) scope. The policy MUST
/// constrain selector results (`trace-publication-tooling` requirement); an
/// empty declared policy therefore constrains selection to the empty set
/// and is never treated as "no constraint declared". A plan with a
/// genuinely empty selection (nothing selected) and an empty policy still
/// passes, since both sides equal the empty set.
///
/// # Errors
///
/// Returns [`SourceGuardError::Stale`] when the selected set contains a path
/// absent from the declared exact policy, or when the declared policy is
/// empty but the selector resolved at least one path.
pub fn enforce_source_set_policy(
    policy: &[String],
    selected: &[String],
) -> Result<(), SourceGuardError> {
    // An exact policy is a two-directional equality: a selector that grew
    // beyond its declared scope, and a declared file the selector no longer
    // resolves, are both scope drift. An empty policy is exact equality
    // with the empty set, never a bypass of this check.
    if selected.iter().any(|path| !policy.contains(path))
        || policy.iter().any(|path| !selected.contains(path))
    {
        return Err(SourceGuardError::Stale);
    }
    Ok(())
}

/// The bounded number of individually reported differing lines. A parity
/// summary is a fixed-size operator detail, not a full diff: an unbounded
/// summary would let a large page dominate the report.
const PARITY_DIFFERING_LINE_LIMIT: usize = 32;

/// Renders one deterministic, fixed-label parity summary between saved and
/// candidate publication bytes.
///
/// # This renderer never echoes document content
///
/// Round-5 finding H1: an earlier revision quoted the differing lines of both
/// documents verbatim. That was unsafe in two independent ways, and the
/// second is a correctness defect rather than a cosmetic one:
///
/// 1. A legitimate publication page routinely contains logical paths, digest
///    payloads, and requirement prose. Echoing that content into a summary
///    that is then checked against TPT-009's forbidden-field contract meant
///    ordinary page bytes could trip the check.
/// 2. Because the caller turned that check failure into a blocked outcome,
///    ARBITRARY EXISTING PAGE BYTES COULD CHANGE THE MACHINE EXIT CLASS --
///    replacing a real immutable conflict with a spurious
///    `TPT-FORBIDDEN-PUBLICATION-FIELD`, and masking the conflict a caller
///    needed to act on.
///
/// The summary therefore reports only renderer-owned metadata: fixed labels,
/// decimal counts, and the line numbers at which the two documents differ.
/// No byte of either document reaches the output, so the forbidden-field
/// property holds BY CONSTRUCTION for every possible input rather than by a
/// check that hostile or merely ordinary content could fail.
/// `parity_metadata_is_clean_for_every_hostile_document` proves this against
/// documents built specifically to smuggle forbidden content.
#[must_use]
pub fn render_parity_diff(existing: Option<&[u8]>, candidate: &[u8]) -> String {
    let Some(existing) = existing else {
        return String::from("existing: absent\ncandidate: present");
    };
    if existing == candidate {
        return String::from("existing: equal\ncandidate: equal");
    }
    let existing_text = String::from_utf8_lossy(existing);
    let candidate_text = String::from_utf8_lossy(candidate);
    let existing_lines: Vec<&str> = existing_text.lines().collect();
    let candidate_lines: Vec<&str> = candidate_text.lines().collect();
    let mut lines = Vec::from([
        String::from("existing: present"),
        String::from("candidate: present"),
        format!("existing: lines {}", existing_lines.len()),
        format!("candidate: lines {}", candidate_lines.len()),
        format!("existing: bytes {}", existing.len()),
        format!("candidate: bytes {}", candidate.len()),
    ]);
    let mut differing = Vec::new();
    for index in 0..existing_lines.len().max(candidate_lines.len()) {
        match (existing_lines.get(index), candidate_lines.get(index)) {
            (Some(left), Some(right)) if left == right => {}
            (left, right) => {
                differing.push((index + 1, left.map(|l| l.len()), right.map(|r| r.len())));
            }
        }
    }
    for (number, left, right) in differing.iter().take(PARITY_DIFFERING_LINE_LIMIT) {
        let left = left.map_or_else(
            || String::from("existing-absent"),
            |bytes| format!("existing-bytes {bytes}"),
        );
        let right = right.map_or_else(
            || String::from("candidate-absent"),
            |bytes| format!("candidate-bytes {bytes}"),
        );
        lines.push(format!("differing-line: {number} {left} {right}"));
    }
    if differing.len() > PARITY_DIFFERING_LINE_LIMIT {
        lines.push(format!(
            "differing-lines-truncated: {}",
            differing.len() - PARITY_DIFFERING_LINE_LIMIT
        ));
    }
    lines.join("\n")
}

/// Every raw platform diff header prefix a deterministic publication parity
/// summary must never contain. A real `diff`/`git diff` header embeds
/// physical paths and modification timestamps, which is precisely what
/// TPT-009 forbids.
/// Exposed so a control can iterate the REAL production list rather than a
/// hand-copied duplicate that silently drifts when an entry is added here.
pub const FORBIDDEN_DIFF_HEADERS: [&str; 5] = ["--- ", "+++ ", "@@ ", "diff --git", "Index: "];

/// Substrings that always identify a physical root, a random temporary
/// artifact, a process identity, or locale data in publication diagnostic
/// text.
/// Exposed for the same reason as [`FORBIDDEN_DIFF_HEADERS`]: the control
/// that proves each entry is load-bearing iterates this exact list, so a new
/// entry cannot be added without also being covered.
pub const FORBIDDEN_PUBLICATION_SUBSTRINGS: [&str; 14] = [
    "/tmp/",
    "/var/folders/",
    "/private/var/",
    ".tmp.",
    ".publication-lock",
    "C:\\",
    "\\Temp\\",
    "pid=",
    "process id",
    "LC_ALL",
    "LANG=",
    "TZ=",
    "$HOME",
    "/Users/",
];

/// Whether one text run looks like a rendered timestamp: an ISO 8601 date
/// (`YYYY-MM-DD`) or a clock time (`HH:MM:SS`).
///
/// A publication diagnostic is byte-identical across roots and times by
/// contract, so any embedded wall-clock value is a determinism defect, not a
/// cosmetic one.
fn contains_timestamp(text: &str) -> bool {
    let bytes: Vec<char> = text.chars().collect();
    // `YYYY-MM-DD`
    for window in bytes.windows(10) {
        if window[0..4].iter().all(char::is_ascii_digit)
            && window[4] == '-'
            && window[5..7].iter().all(char::is_ascii_digit)
            && window[7] == '-'
            && window[8..10].iter().all(char::is_ascii_digit)
        {
            return true;
        }
    }
    // `HH:MM:SS`
    for window in bytes.windows(8) {
        if window[0..2].iter().all(char::is_ascii_digit)
            && window[2] == ':'
            && window[3..5].iter().all(char::is_ascii_digit)
            && window[5] == ':'
            && window[6..8].iter().all(char::is_ascii_digit)
        {
            return true;
        }
    }
    false
}

/// Validates one rendered publication diagnostic or parity summary against
/// TPT-009's forbidden-field contract.
///
/// Publication diagnostics are the automation contract: two runs over equal
/// logical inputs under different physical roots, at different times, in
/// different locales, must produce byte-identical output. Any timestamp,
/// random temporary name, physical root, process ID, locale value, or raw
/// platform diff header breaks that guarantee, so this validator rejects
/// each one with the same stable code.
///
/// # Errors
///
/// Returns [`PublicationError::InvalidField`] carrying the
/// `TPT-FORBIDDEN-PUBLICATION-FIELD` stable code for any forbidden content.
pub fn validate_publication_diagnostic_text(text: &str) -> Result<(), PublicationError> {
    let forbidden = || PublicationError::InvalidField("forbidden-publication-field".into());
    for line in text.lines() {
        if FORBIDDEN_DIFF_HEADERS
            .iter()
            .any(|header| line.starts_with(header))
        {
            return Err(forbidden());
        }
    }
    if FORBIDDEN_PUBLICATION_SUBSTRINGS
        .iter()
        .any(|needle| text.contains(needle))
    {
        return Err(forbidden());
    }
    if contains_timestamp(text) {
        return Err(forbidden());
    }
    Ok(())
}

/// The stable diagnostic codes publication Markdown projection conformance
/// reports.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ProjectionError {
    /// The projection labels a source-graph digest ambiguously (as `Graph`
    /// rather than `Source graph`), so a reader cannot tell which graph
    /// domain the value belongs to.
    AmbiguousGraphLabel,
    /// The projection omits its own renderer schema or the machine
    /// trace-report schema it derives from.
    MissingSchemaIdentity,
    /// A relation row omits the complete semantic atom revision.
    MissingAtomRevision,
    /// A relation row omits a required freshness facet.
    MissingFreshnessFacet,
    /// A relation row omits one of the two required logical locations.
    MissingLogicalLocation,
    /// The projection omits the derived-data warning or the navigation-only
    /// assurance boundary.
    MissingBoundary,
}

impl ProjectionError {
    /// The stable `RTP-*` diagnostic code this projection fault reports.
    #[must_use]
    pub const fn stable_code(self) -> &'static str {
        match self {
            Self::AmbiguousGraphLabel => "RTP-AMBIGUOUS-GRAPH-LABEL",
            Self::MissingSchemaIdentity => "RTP-MISSING-SCHEMA-IDENTITY",
            Self::MissingAtomRevision => "RTP-MISSING-ATOM-REVISION",
            Self::MissingFreshnessFacet => "RTP-MISSING-FRESHNESS-FACET",
            Self::MissingLogicalLocation => "RTP-MISSING-LOGICAL-LOCATION",
            Self::MissingBoundary => "RTP-MISSING-BOUNDARY",
        }
    }
}

/// The five row-local freshness facet labels every rendered relation row
/// must expose.
pub const REQUIRED_FRESHNESS_FACETS: [&str; 5] = [
    "record",
    "attachment",
    "implementation",
    "requirement",
    "authority",
];

/// Runs publication projection conformance over one rendered
/// `buoy/readable-trace-markdown/v1` document.
///
/// This is the real conformance entry point RTP-007's negative scenarios
/// name. It runs against rendered bytes rather than the typed report, so it
/// catches a renderer that silently drops a required label or facet even
/// though the underlying `buoy/trace-report/v1` value still carries it --
/// exactly the drift an immutable publication basis must never key.
///
/// # Errors
///
/// Returns a deterministic [`ProjectionError`] for an ambiguous graph label,
/// a missing schema identity, a row missing its atom revision, a missing
/// freshness facet, a missing logical location, or a dropped assurance
/// boundary.
pub fn validate_markdown_projection(text: &str) -> Result<(), ProjectionError> {
    // A source-graph digest MUST be labelled unambiguously. A bare `Graph:`
    // label names no domain: a reader cannot tell a source graph from a
    // binding graph or a page graph. Detect the ambiguous spelling directly
    // rather than only checking that the unambiguous one is present, so a
    // projection carrying BOTH labels still fails.
    for line in text.lines() {
        let trimmed = line.trim_start().trim_start_matches("- ");
        if trimmed.starts_with("Graph:") || trimmed.starts_with("Graph :") {
            return Err(ProjectionError::AmbiguousGraphLabel);
        }
    }
    if !text.contains("Source graph") {
        return Err(ProjectionError::AmbiguousGraphLabel);
    }
    if !text.contains("buoy/readable-trace-markdown/v1") || !text.contains("buoy/trace-report/v1") {
        return Err(ProjectionError::MissingSchemaIdentity);
    }
    if !text.contains("Derived data")
        || !text.contains("navigation and impact only")
        || !text.contains("Buoy remains the current-state authority")
    {
        return Err(ProjectionError::MissingBoundary);
    }
    // The relation table's header row declares which row-local facts each
    // rendered row exposes. A renderer that drops a column drops that fact
    // from every row at once, so checking the declared header is the exact
    // row-completeness check RTP-007 requires.
    let header = text
        .lines()
        .find(|line| line.starts_with("| State |"))
        .ok_or(ProjectionError::MissingAtomRevision)?;
    if !header.contains("Atom revision") {
        return Err(ProjectionError::MissingAtomRevision);
    }
    if !header.contains("Requirement path") || !header.contains("Implementation path") {
        return Err(ProjectionError::MissingLogicalLocation);
    }
    if !header.contains("Freshness") {
        return Err(ProjectionError::MissingFreshnessFacet);
    }
    // Every rendered relation row must carry all five freshness facets by
    // name. A row that silently omits one (for example `attachment`) still
    // renders a plausible-looking `Freshness` cell, so the header check
    // above cannot catch it; only scanning each real row can.
    for line in text.lines() {
        if line.starts_with("|---") || !line.starts_with("| ") || line.starts_with("| State |") {
            continue;
        }
        if REQUIRED_FRESHNESS_FACETS
            .iter()
            .any(|facet| !line.contains(&format!("{facet}=")))
        {
            return Err(ProjectionError::MissingFreshnessFacet);
        }
    }
    Ok(())
}

/// Rejects two declared output paths that would collide on one target.
///
/// # Errors
///
/// Returns a deterministic diagnostic when any two normalized paths are
/// byte-identical.
pub fn detect_path_collisions(paths: &[&str]) -> Result<(), PublicationError> {
    let mut seen = BTreeSet::new();
    for path in paths {
        if !seen.insert(normalize_path(path)) {
            return Err(PublicationError::InvalidField("output-collision".into()));
        }
    }
    Ok(())
}

/// Whether two logical paths name the same repository-relative location
/// after normalizing a leading `./` segment and collapsing doubled slashes.
///
/// This closes the alias gap a bare string comparison misses: `spec/a.md`
/// and `./spec/a.md` must classify identically.
fn paths_alias(left: &str, right: &str) -> bool {
    normalize_path(left) == normalize_path(right)
}

fn normalize_path(path: &str) -> String {
    path.split('/')
        .filter(|segment| !segment.is_empty() && *segment != ".")
        .collect::<Vec<_>>()
        .join("/")
}

/// One requested Buoy consumer-package variant and its declared inputs.
#[derive(Clone, Debug)]
pub struct PackageVariant<'a> {
    pub features: Vec<&'a str>,
    pub source_revision: &'a str,
    pub ambient_cargo_home: Option<&'a str>,
}

const PACKAGE_FEATURES: [&str; 3] = ["xxh3-128", "blake3-256", "sha-256"];

/// Validates one package variant against the closed version 1 feature set.
///
/// # Errors
///
/// Returns a deterministic diagnostic for an empty source revision, an
/// unknown feature, or ambient host tool leakage into the build environment.
pub fn validate_package_variant(variant: &PackageVariant<'_>) -> Result<(), PublicationError> {
    if variant.source_revision.is_empty() {
        return Err(PublicationError::InvalidField(
            "package-source-identity".into(),
        ));
    }
    if variant.features.is_empty()
        || variant
            .features
            .iter()
            .any(|feature| !PACKAGE_FEATURES.contains(feature))
    {
        return Err(PublicationError::InvalidField("package-feature".into()));
    }
    if variant.ambient_cargo_home.is_some() {
        return Err(PublicationError::InvalidField(
            "package-ambient-tool".into(),
        ));
    }
    Ok(())
}

/// Historical `OpenSpec` completion-evidence Markdown block, if present.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct CompletionEvidence {
    pub publication_id: Option<String>,
}

const EVIDENCE_BEGIN: &str = "<!-- BEGIN buoy-trace-publication-evidence-v1 -->";
const EVIDENCE_END: &str = "<!-- END buoy-trace-publication-evidence-v1 -->";

/// Placeholder text a real generated identity value can never equal.
const COMPLETION_PLACEHOLDERS: [&str; 5] = ["none", "n/a", "tbd", "pending", "todo"];

/// Whether one generated-block value is a placeholder stand-in rather than a
/// real identity: a closed placeholder word (case-insensitive) or an
/// angle-bracketed template token such as `<publication-id>`.
fn is_placeholder_identity(value: &str) -> bool {
    let lowered = value.to_ascii_lowercase();
    COMPLETION_PLACEHOLDERS.contains(&lowered.as_str())
        || (value.starts_with('<') && value.ends_with('>'))
        || value.is_empty()
}

/// Parses one marker-delimited generated completion-evidence block.
///
/// Rejects a manual record row that conflicts with generated evidence and
/// rejects prose that claims the document itself is the current state.
///
/// # Errors
///
/// Returns a deterministic diagnostic for a drifted or scope-violating
/// completion document.
pub fn parse_completion_evidence(text: &str) -> Result<CompletionEvidence, PublicationError> {
    if text.contains("currently represents the live state")
        || text.contains("No further query is needed")
    {
        return Err(PublicationError::InvalidField("claim-boundary".into()));
    }
    let Some(begin) = text.find(EVIDENCE_BEGIN) else {
        return Ok(CompletionEvidence::default());
    };
    let Some(end) = text[begin..].find(EVIDENCE_END) else {
        return Err(PublicationError::InvalidField("completion-evidence".into()));
    };
    let block = &text[begin + EVIDENCE_BEGIN.len()..begin + end];
    let publication_id = block
        .lines()
        .find_map(|line| line.strip_prefix("Publication ID: "))
        .map(str::trim)
        .map(str::to_owned);
    // A generated block MUST NOT carry a placeholder identity: real
    // publication tooling never emits "TBD", "none", "pending", or an
    // angle-bracketed template token in place of a real value. Before any
    // implementation exists, the generated block MUST stay entirely empty
    // (no `Publication ID:` line at all) rather than carry a fabricated
    // stand-in identity.
    if let Some(id) = &publication_id {
        if is_placeholder_identity(id) {
            return Err(PublicationError::InvalidField(
                "completion-evidence-placeholder".into(),
            ));
        }
    }
    let generated_record = block
        .lines()
        .find_map(|line| line.strip_prefix("Record: "))
        .map(str::trim);
    if let Some(record) = generated_record {
        if is_placeholder_identity(record) {
            return Err(PublicationError::InvalidField(
                "completion-evidence-placeholder".into(),
            ));
        }
    }
    if let Some(generated) = generated_record {
        // A manual conflict row can name its digest under ANY registered
        // digest-registry member, not only `blake3-256`: hardcoding one
        // algorithm's prefix would silently miss a conflicting row under
        // `xxh3-128` or `sha-256`.
        let manual_conflict = text[begin + end..].lines().any(|line| {
            line.trim_start().starts_with('|')
                && crate::core::DigestAlgorithm::ALL
                    .iter()
                    .any(|algorithm| line.contains(&format!("{}:", algorithm.canonical_name())))
                && !line.contains(generated)
                && !line.contains("Canonical record locator")
        });
        if manual_conflict {
            return Err(PublicationError::InvalidField(
                "completion-evidence-drift".into(),
            ));
        }
    }
    Ok(CompletionEvidence { publication_id })
}

/// Omits only the diagnostics whose primary stable ID equals the declared
/// publication link's ID, and retains every other diagnostic unchanged.
///
/// Pre-link conformance must never treat a stale or absent declared link as
/// a blocker for the exact link generation about to repair it, but it must
/// still block on every unrelated relation, requirement, implementation,
/// authority, or other external-link diagnostic.
#[must_use]
pub fn omit_declared_link_diagnostics(
    diagnostics: &[crate::check::ShellDiagnostic],
    link_id: &str,
) -> Vec<crate::check::ShellDiagnostic> {
    diagnostics
        .iter()
        .filter(|diagnostic| diagnostic.id.as_deref() != Some(link_id))
        .cloned()
        .collect()
}

/// Computes the `buoy/trace-publication-basis/v1` digest under one registered
/// digest-registry member.
///
/// This calls the same repository digest registry every other Buoy identity
/// uses (`crate::core::ActiveAlgorithm::compute`); it never implements a
/// private hash. The algorithm must be compiled into this build.
///
/// # Errors
///
/// Returns [`PublicationError::DigestMemberUnavailable`] when the requested
/// member has no build adapter.
pub fn publication_basis_digest(
    algorithm: crate::core::DigestAlgorithm,
    bytes: &[u8],
) -> Result<String, PublicationError> {
    domain_digest(algorithm, b"buoy/trace-publication-basis/v1", bytes)
}

/// Requires an immutable basis or page-content digest to use a member with
/// the `collision-resistant` assurance class.
///
/// # Errors
///
/// Returns [`PublicationError::PublicationDigestClass`] for a
/// change-detection member such as `xxh3-128`.
pub fn require_collision_resistant_member(
    algorithm: crate::core::DigestAlgorithm,
) -> Result<(), PublicationError> {
    if algorithm.assurance_class() == crate::core::DigestAssuranceClass::CollisionResistant {
        Ok(())
    } else {
        Err(PublicationError::PublicationDigestClass)
    }
}

/// Every complete pre-link input the publication basis binds.
///
/// `normalized_snapshot` MUST already have had every declared publication
/// slot removed and replaced with its fixed, link-ID-keyed marker (see
/// [`crate::external_link_generation::normalize_for_basis`]) for every path
/// this snapshot selects that carries a publication slot. Selected paths and
/// their bytes MUST already be in canonical (sorted-by-path) order; this
/// function does not re-sort them, so callers own that ordering guarantee.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BasisInputs<'a> {
    pub plan_schema: &'a str,
    pub renderer_schema: &'a str,
    pub active_member: &'a str,
    pub selector: &'a Selector,
    pub trace_rows: &'a [TraceRow],
    pub normalized_snapshot: &'a [(String, Vec<u8>)],
}

/// Computes the complete `buoy/trace-publication-basis/v1` digest from every
/// declared pre-link input, encoded in one fixed canonical byte layout.
///
/// The encoding is order-sensitive only in the caller-supplied
/// `normalized_snapshot` ordering; this function performs no filesystem
/// traversal and depends on no ambient root, locale, or clock, so equal
/// inputs under any physical root or platform produce byte-identical output.
///
/// # Errors
///
/// Returns [`PublicationError::InvalidField`] when any declared trace row is
/// incomplete, since an incomplete row cannot contribute a stable basis
/// input.
pub fn compute_publication_basis(inputs: &BasisInputs<'_>) -> Result<String, PublicationError> {
    let algorithm = crate::core::DigestAlgorithm::from_canonical_name(inputs.active_member)
        .ok_or(PublicationError::DigestMemberUnavailable)?;
    let mut canonical = Vec::new();
    canonical.extend_from_slice(inputs.plan_schema.as_bytes());
    canonical.push(0);
    canonical.extend_from_slice(inputs.renderer_schema.as_bytes());
    canonical.push(0);
    canonical.extend_from_slice(inputs.active_member.as_bytes());
    canonical.push(0);
    canonical.extend_from_slice(selector_canonical_bytes(inputs.selector).as_slice());
    canonical.push(0);
    for row in inputs.trace_rows {
        if !row.is_complete() {
            return Err(PublicationError::InvalidField(
                "incomplete trace row in basis input".into(),
            ));
        }
        for field in [
            row.relation.as_deref(),
            row.record.as_deref(),
            row.requirement_id.as_deref(),
            row.atom_revision.as_deref(),
            row.requirement_path.as_deref(),
            row.implementation_path.as_deref(),
            row.state.as_deref(),
        ] {
            canonical.extend_from_slice(field.unwrap_or_default().as_bytes());
            canonical.push(0x1f);
        }
        canonical.push(0x1e);
    }
    canonical.push(0);
    for (path, bytes) in inputs.normalized_snapshot {
        canonical.extend_from_slice(path.as_bytes());
        canonical.push(0x1f);
        canonical.extend_from_slice(&(bytes.len() as u64).to_le_bytes());
        canonical.extend_from_slice(bytes);
        canonical.push(0x1e);
    }
    publication_basis_digest(algorithm, &canonical)
}

fn selector_canonical_bytes(selector: &Selector) -> Vec<u8> {
    match selector {
        Selector::RequirementId(value) => format!("requirement_id={value}").into_bytes(),
        Selector::RelationId(value) => format!("relation_id={value}").into_bytes(),
        Selector::ImplementationPath(value) => format!("implementation_path={value}").into_bytes(),
        Selector::Record(value) => format!("record={value}").into_bytes(),
    }
}
/// Computes the `buoy/trace-publication-content/v1` digest under one
/// registered digest-registry member.
///
/// # Errors
///
/// Returns [`PublicationError::DigestMemberUnavailable`] when the requested
/// member has no build adapter.
pub fn page_content_digest(
    algorithm: crate::core::DigestAlgorithm,
    bytes: &[u8],
) -> Result<String, PublicationError> {
    domain_digest(algorithm, b"buoy/trace-publication-content/v1", bytes)
}

fn domain_digest(
    algorithm: crate::core::DigestAlgorithm,
    domain: &[u8],
    bytes: &[u8],
) -> Result<String, PublicationError> {
    #[cfg(feature = "coverage-controls")]
    let active = if bytes == b"coverage-unavailable-member" {
        None
    } else {
        crate::core::ActiveAlgorithm::new(algorithm).ok()
    };
    #[cfg(not(feature = "coverage-controls"))]
    let active = crate::core::ActiveAlgorithm::new(algorithm).ok();
    let Some(active) = active else {
        return Err(PublicationError::DigestMemberUnavailable);
    };
    let mut preimage = Vec::with_capacity(domain.len() + bytes.len());
    preimage.extend_from_slice(domain);
    preimage.extend_from_slice(bytes);
    active
        .compute(algorithm, &preimage)
        .map(|digest| digest.canonical_text())
        .map_err(|_| PublicationError::DigestMemberUnavailable)
}

/// Joins a normalized absolute `https` target base with one rendered
/// logical page path.
///
/// The base must end with `/` and carry no query or fragment. The join
/// percent-encodes normalized UTF-8 path segments, preserves slash
/// boundaries, and rejects an encoded or literal `..` traversal segment.
///
/// # Errors
///
/// Returns [`PublicationError::InvalidTargetBase`] for a malformed base, and
/// [`PublicationError::InvalidField`] for a traversal segment in the page
/// path.
pub fn join_target_base(base: &str, page_path: &str) -> Result<String, PublicationError> {
    validate_target_base(base)?;
    if page_path
        .split('/')
        .any(|segment| segment.is_empty() || segment == "..")
    {
        return Err(PublicationError::InvalidField("target-traversal".into()));
    }
    let encoded = page_path
        .split('/')
        .map(percent_encode_segment)
        .collect::<Vec<_>>()
        .join("/");
    Ok(format!("{base}{encoded}"))
}

fn percent_encode_segment(segment: &str) -> String {
    use std::fmt::Write as _;
    let mut encoded = String::with_capacity(segment.len());
    for byte in segment.bytes() {
        match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'.' | b'_' | b'~' => {
                encoded.push(byte as char);
            }
            _ => {
                let _ = write!(encoded, "%{byte:02X}");
            }
        }
    }
    encoded
}

/// Requires one immutable target to contain the exact publication-basis
/// path encoding.
///
/// Atom-revision and binding-record path encodings can add stable grouping,
/// but only the basis encoding satisfies the immutable-key completeness
/// requirement.
///
/// # Errors
///
/// Returns [`PublicationError::IncompletePublicationKey`] when the target
/// omits the basis encoding.
pub fn validate_immutable_target(
    target: &str,
    basis_path_encoding: &str,
) -> Result<(), PublicationError> {
    if target.contains(basis_path_encoding) {
        Ok(())
    } else {
        Err(PublicationError::IncompletePublicationKey)
    }
}

#[cfg(all(test, feature = "coverage-controls"))]
#[allow(clippy::expect_used, clippy::too_many_lines)]
mod coverage_tests {
    use super::*;

    fn raw_output(kind: &str, lifecycle: &str) -> RawOutput {
        RawOutput {
            id: String::from("output"),
            mount: String::from("repository"),
            path: String::from("out/{publication_basis_path}.md"),
            kind: String::from(kind),
            lifecycle: String::from(lifecycle),
            relation: None,
            page: None,
            target_base: None,
        }
    }

    fn complete_row() -> TraceRow {
        TraceRow {
            relation: Some(String::from("RELATION-1")),
            record: Some(String::from("xxh3-128:11111111111111111111111111111111")),
            requirement_id: Some(String::from("REQ-001")),
            atom_revision: Some(String::from("xxh3-128:22222222222222222222222222222222")),
            requirement_node: Some(String::from("xxh3-128:33333333333333333333333333333333")),
            implementation_node: Some(String::from("xxh3-128:44444444444444444444444444444444")),
            requirement_path: Some(String::from("openspec/specs/demo/spec.md")),
            implementation_path: Some(String::from("src/lib.rs")),
            state: Some(String::from("current")),
            freshness: Some(RowFreshness {
                record: true,
                attachment: true,
                implementation: true,
                requirement: true,
                authority: true,
            }),
            diagnostics: Vec::new(),
        }
    }

    fn plan_json(selector: &Value, outputs: &Value) -> Vec<u8> {
        serde_json::to_vec(&serde_json::json!({
            "schema": PLAN_SCHEMA,
            "publication_id": "coverage-plan",
            "selector": selector,
            "expected_rows": 1,
            "source_set": ["src/lib.rs"],
            "outputs": outputs,
        }))
        .expect("plan JSON")
    }

    #[test]
    fn duplicate_key_guard_walks_every_json_value_class() {
        for value in [
            b"true".as_slice(),
            b"-1".as_slice(),
            b"1".as_slice(),
            b"1.5".as_slice(),
            br#""text""#.as_slice(),
            b"null".as_slice(),
            br#"[true, -1, 1, 1.5, "text", null, {}]"#.as_slice(),
        ] {
            assert!(serde_json::from_slice::<DuplicateKeyGuard>(value).is_ok());
            assert!(parse_plan(value).is_err());
        }
        assert!(
            parse_plan(br#"{"selector":{"requirement_id":"A","requirement_id":"B"}}"#).is_err()
        );
    }

    #[test]
    fn publication_error_codes_cover_the_closed_failure_catalog() {
        let errors = vec![
            PublicationError::InvalidJson,
            PublicationError::MissingField(String::new()),
            PublicationError::UnknownField(String::new()),
            PublicationError::OrphanImmutablePage,
            PublicationError::InvalidTargetBase,
            PublicationError::IncompletePublicationKey,
            PublicationError::PlanBytesBudget,
            PublicationError::PlanOutputBudget,
            PublicationError::PlanSourceSetBudget,
            PublicationError::ConflictingSelector,
            PublicationError::DigestMemberUnavailable,
            PublicationError::PublicationDigestClass,
        ];
        for error in &errors {
            assert!(!error.stable_code().is_empty());
            assert_eq!(
                error.exit_class(),
                if matches!(error, &PublicationError::ConflictingSelector) {
                    64
                } else {
                    70
                }
            );
        }
        for reason in [
            "path-traversal",
            "substitution",
            "substitution-unavailable",
            "output-escape",
            "output-link",
            "output-collision",
            "selected-output",
            "source-edit-not-selected",
            "target-base-not-page-owned",
            "output-path-no-substitution",
            "package-source-identity",
            "package-feature",
            "package-ambient-tool",
            "completion-evidence",
            "completion-evidence-drift",
            "completion-evidence-placeholder",
            "claim-boundary",
            "external-link relation",
            "external-link-missing-page",
            "external-link-unresolved-page",
            "external-link-multiple-owners",
            "duplicate output",
            "target-traversal",
            "report",
            "package identity",
            "source-set-entry",
            "source-set-duplicate",
            "target-base-missing",
            "duplicate-json-key",
            "output-id",
            "forbidden-publication-field",
            "other",
        ] {
            assert!(
                !PublicationError::InvalidField(String::from(reason))
                    .stable_code()
                    .is_empty()
            );
        }
    }

    #[test]
    fn output_and_plan_parsers_reject_each_untrusted_shape() {
        assert!(validate_path_template_substitutions("../page.md").is_err());
        assert!(validate_path_template_substitutions("out/{unterminated").is_err());
        for (kind, lifecycle) in [("unknown", "replaceable"), ("page", "unknown")] {
            assert!(parse_output(raw_output(kind, lifecycle)).is_err());
        }
        let mut output = raw_output("page", "replaceable");
        output.id.clear();
        assert!(parse_output(output).is_err());
        let mut output = raw_output("page", "replaceable");
        output.path.clear();
        assert!(parse_output(output).is_err());
        let mut output = raw_output("page", "replaceable");
        output.mount.clear();
        assert!(parse_output(output).is_err());
        assert!(parse_output(raw_output("external-link", "replaceable")).is_err());
        let mut output = raw_output("external-link", "replaceable");
        output.relation = Some(String::from("documents"));
        assert!(parse_output(output).is_err());
        let mut output = raw_output("page", "immutable");
        output.path = String::from("out/page.md");
        assert!(parse_output(output).is_err());
        let mut output = raw_output("source-edit", "replaceable");
        output.path = String::from("spec/{publication_basis}.md");
        assert!(parse_output(output).is_err());
        let mut output = raw_output("source-edit", "replaceable");
        output.path = String::from("spec/unmatched}.md");
        assert!(parse_output(output).is_err());
        let mut output = raw_output("source-edit", "replaceable");
        output.path = String::from("spec/source.md");
        output.target_base = Some(String::from("https://example.com/"));
        assert!(parse_output(output).is_err());
        for invalid in [
            "https://example.com/path?query/",
            "https://example.com/path#fragment/",
            "https://example.com/{path}/",
            "https://example.com/path}/",
            "https://example.com/a b/",
            "https://example.com/a\\b/",
            "https:///",
            "https://user@example.com/path/",
            "https://example.com:/path/",
            "https://example.com/path//",
            "https://example.com/../path/",
            "https://example.com/%2e%2e/path/",
        ] {
            assert!(validate_target_base(invalid).is_err(), "{invalid}");
        }
        assert!(validate_target_base("https://example.com/path/").is_ok());

        assert!(
            selector(RawSelector {
                requirement_id: None,
                relation_id: None,
                implementation_path: None,
                record: None,
            })
            .is_err()
        );
        let malformed = plan_json(
            &serde_json::json!({"requirement_id": "REQ-001"}),
            &serde_json::json!([{"id":"output","mount":"repository","path":"out/page.md","kind":"unknown","lifecycle":"replaceable"}]),
        );
        assert!(parse_plan(&malformed).is_err());
        let bounded = String::from_utf8(plan_json(
            &serde_json::json!({"requirement_id": "REQ-001"}),
            &serde_json::json!([]),
        ))
        .expect("plan text");
        let wrong_schema = bounded.replace(PLAN_SCHEMA, "wrong");
        assert!(parse_plan(wrong_schema.as_bytes()).is_err());
        assert!(parse_plan(b"{").is_err());
        assert!(
            parse_plan(
                bounded
                    .replacen("\"expected_rows\":1", "\"expected_rows\":0", 1)
                    .as_bytes()
            )
            .is_err()
        );
        assert!(
            parse_plan(
                bounded
                    .replacen(
                        "\"publication_id\":\"coverage-plan\"",
                        "\"publication_id\":\"\"",
                        1
                    )
                    .as_bytes()
            )
            .is_err()
        );

        let duplicate_id = plan_json(
            &serde_json::json!({"requirement_id": "REQ-001"}),
            &serde_json::json!([
                {"id":"same","mount":"repository","path":"out/one.md","kind":"page","lifecycle":"replaceable"},
                {"id":"same","mount":"repository","path":"out/two.md","kind":"page","lifecycle":"replaceable"}
            ]),
        );
        assert!(parse_plan(&duplicate_id).is_err());
        let duplicate_path = plan_json(
            &serde_json::json!({"requirement_id": "REQ-001"}),
            &serde_json::json!([
                {"id":"one","mount":"repository","path":"out/page.md","kind":"page","lifecycle":"replaceable"},
                {"id":"two","mount":"repository","path":"out/page.md","kind":"page","lifecycle":"replaceable"}
            ]),
        );
        assert!(parse_plan(&duplicate_path).is_err());

        let immutable = TracePublicationPlan {
            schema: String::from(PLAN_SCHEMA),
            publication_id: String::from("coverage-plan"),
            selector: Selector::RequirementId(String::from("REQ-001")),
            expected_rows: 1,
            source_set: vec![String::from("src/lib.rs")],
            outputs: vec![PublicationOutput {
                id: String::from("page"),
                mount: String::from("repository"),
                path: String::from("out/{publication_basis_path}.md"),
                kind: ArtifactKind::Page,
                lifecycle: Lifecycle::Immutable,
                relation: None,
                page: None,
                target_base: None,
            }],
        };
        assert_eq!(immutable.immutable_page_count(), 1);
        let mut replaceable = immutable.clone();
        replaceable.outputs[0].lifecycle = Lifecycle::Replaceable;
        assert_eq!(replaceable.immutable_page_count(), 0);
        assert_eq!(immutable.schema(), PLAN_SCHEMA);
        assert_eq!(immutable.publication_id(), "coverage-plan");
        assert_eq!(immutable.expected_rows(), 1);
        assert_eq!(immutable.source_set(), ["src/lib.rs"]);
        assert_eq!(immutable.outputs().len(), 1);
    }

    #[test]
    fn report_and_evidence_accessors_render_every_optional_field() {
        let report = PublicationReport::new("check", "page")
            .with_plan_id("plan")
            .with_basis("xxh3-128:basis")
            .with_graph("xxh3-128:graph")
            .with_prior("unequal", Some("xxh3-128:prior"))
            .with_candidate_digest("xxh3-128:candidate")
            .with_outcome("blocked")
            .with_diagnostic("TPT-CONTROL", "controlled diagnostic")
            .with_parity(Some(b"old\n"), b"new\n");
        assert_eq!(report.operation(), "check");
        assert_eq!(report.artifact(), "page");
        assert_eq!(report.plan_id(), Some("plan"));
        assert_eq!(report.basis(), Some("xxh3-128:basis"));
        assert_eq!(report.graph(), Some("xxh3-128:graph"));
        assert_eq!(report.prior_state(), Some("unequal"));
        assert_eq!(report.prior_digest(), Some("xxh3-128:prior"));
        assert_eq!(report.candidate_digest(), Some("xxh3-128:candidate"));
        assert_eq!(report.outcome(), Some("blocked"));
        assert_eq!(report.diagnostics().len(), 1);
        assert!(report.parity().is_some());
        assert!(render_report(&report).is_ok());
        assert!(
            render_report_human(&report)
                .expect("human report")
                .contains("parity:")
        );
        assert_eq!(
            validated_parity(String::from("/tmp/forbidden")),
            "parity: unavailable (renderer metadata rejected)"
        );
        assert!(render_report(&PublicationReport::new("", "page")).is_err());
        assert!(
            render_report(&PublicationReport::new("check", "page").with_scope_claim("remote"))
                .is_err()
        );
        assert!(
            render_report(
                &PublicationReport::new("check", "page")
                    .with_diagnostic("TPT-CONTROL", "/tmp/forbidden")
            )
            .is_err()
        );
        assert!(
            render_report(
                &PublicationReport::new("check", "page")
                    .with_diagnostic("/tmp/forbidden-code", "controlled")
            )
            .is_err()
        );
        let invalid_parity = PublicationReport {
            parity: Some(String::from("/tmp/forbidden-parity")),
            ..PublicationReport::new("check", "page")
        };
        assert!(render_report_human(&invalid_parity).is_err());

        let row = complete_row();
        for selector in [
            Selector::RequirementId(String::from("REQ-001")),
            Selector::RelationId(String::from("RELATION-1")),
            Selector::ImplementationPath(String::from("src/lib.rs")),
            Selector::Record(String::from("xxh3-128:record")),
        ] {
            let evidence = PublicationEvidence::new("publication")
                .with_source_digest("xxh3-128:source")
                .with_selector(&selector)
                .with_basis("xxh3-128:basis")
                .with_graph("xxh3-128:graph")
                .with_rows(core::slice::from_ref(&row))
                .with_link_target("https://example.com/trace")
                .with_page_digest("xxh3-128:page")
                .with_diagnostic("TPT-CONTROL", "controlled diagnostic");
            assert_eq!(evidence.publication_id(), "publication");
            assert_eq!(evidence.source_digest(), Some("xxh3-128:source"));
            assert!(evidence.selector_text().is_some());
            assert_eq!(evidence.basis(), Some("xxh3-128:basis"));
            assert_eq!(evidence.graph(), Some("xxh3-128:graph"));
            assert_eq!(evidence.rows(), core::slice::from_ref(&row));
            assert_eq!(evidence.link_target(), Some("https://example.com/trace"));
            assert_eq!(evidence.page_digest(), Some("xxh3-128:page"));
            assert_eq!(evidence.diagnostics().len(), 1);
            assert_eq!(evidence.status(), EvidenceStatus::Blocked);
            assert!(evidence.render_markdown().contains("External-link target"));
            assert!(!render_evidence(&evidence).is_empty());
        }
        let empty = PublicationEvidence::new("");
        assert!(empty.render_markdown().is_empty());
        assert!(render_evidence(&empty).is_empty());
    }

    #[test]
    fn package_path_row_and_projection_controls_cover_all_outcomes() {
        for identity in [
            PackageIdentity::new("", "revision", "toolchain", vec!["xxh3-128"]),
            PackageIdentity::new("1", "", "toolchain", vec!["xxh3-128"]),
            PackageIdentity::new("1", "revision", "", vec!["xxh3-128"]),
            PackageIdentity::new("1", "revision", "toolchain", Vec::new()),
            PackageIdentity::new("1", "revision", "toolchain", vec![""]),
        ] {
            assert!(identity.render().is_err());
        }
        let identity = PackageIdentity::new("1", "revision", "toolchain", vec!["xxh3-128"]);
        assert_eq!(identity.source_revision(), "revision");
        assert!(identity.render().is_ok());
        assert_eq!(
            page_content_digest(
                crate::core::DigestAlgorithm::Xxh3_128,
                b"coverage-unavailable-member",
            ),
            Err(PublicationError::DigestMemberUnavailable)
        );

        let identities = PathIdentities {
            basis: "xxh3-128:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
            atom_revision: Some("xxh3-128:bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb"),
            record_digest: Some("xxh3-128:cccccccccccccccccccccccccccccccc"),
        };
        for template in [
            "out/{publication_basis_path}.md",
            "out/{publication_basis}.md",
            "out/{atom_revision_path}.md",
            "out/{atom_revision}.md",
            "out/{record_digest_path}.md",
            "out/{record_digest}.md",
        ] {
            assert!(render_path(template, &identities).is_ok());
        }
        assert!(render_path("out/{unknown}.md", &identities).is_err());
        assert!(render_path("out/{unterminated", &identities).is_err());
        assert!(render_path("../out.md", &identities).is_err());
        assert!(
            render_path(
                "out/{atom_revision}.md",
                &PathIdentities {
                    basis: identities.basis,
                    atom_revision: None,
                    record_digest: identities.record_digest,
                },
            )
            .is_err()
        );

        let row = complete_row();
        let fragmenters: [fn(&mut TraceRow); 9] = [
            |row| row.relation = None,
            |row| row.record = None,
            |row| row.requirement_id = None,
            |row| row.atom_revision = None,
            |row| row.requirement_node = None,
            |row| row.implementation_node = None,
            |row| row.requirement_path = None,
            |row| row.implementation_path = None,
            |row| row.freshness = None,
        ];
        for fragment in fragmenters {
            let mut fragmented = row.clone();
            fragment(&mut fragmented);
            assert!(!fragmented.is_complete());
        }
        assert_eq!(row_conformance(core::slice::from_ref(&row), 1), Ok(()));
        assert_eq!(row_conformance(&[], 1), Err(ConformanceError::RowCount));
        assert_eq!(
            row_conformance(&[TraceRow::default()], 1),
            Err(ConformanceError::RowFragmentation)
        );
        let mut stale = row.clone();
        stale.state = Some(String::from("stale"));
        assert_eq!(row_conformance(&[stale], 1), Err(ConformanceError::Stale));
        let mut diagnosed = row.clone();
        diagnosed.diagnostics.push(String::from("CAB-CONTROL"));
        assert_eq!(
            row_conformance(&[diagnosed], 1),
            Err(ConformanceError::RowDiagnostic)
        );
        for error in [
            ConformanceError::RowCount,
            ConformanceError::RowFragmentation,
            ConformanceError::Stale,
            ConformanceError::RowDiagnostic,
        ] {
            assert!(!error.stable_code().is_empty());
        }
        for path in [
            "",
            "/absolute",
            "out/../page.md",
            "out/./page.md",
            "out//page.md",
            "out/link",
            "out/link/page.md",
        ] {
            assert!(classify_output(path, "derived", &[]).is_err(), "{path}");
        }
        assert_eq!(
            classify_output(
                "spec/source.md",
                "source-edit",
                &[String::from("./spec//source.md")]
            ),
            Ok(OutputClass::SourceEdit)
        );
        assert!(classify_output("spec/other.md", "source-edit", &[]).is_err());
        assert!(
            classify_output(
                "spec/source.md",
                "derived",
                &[String::from("spec/source.md")]
            )
            .is_err()
        );
        assert_eq!(
            classify_output("out/page.md", "derived", &[]),
            Ok(OutputClass::Derived)
        );
        assert!(detect_path_collisions(&["./out//page.md", "out/page.md"]).is_err());

        for timestamp in ["2026-08-26", "23:59:58"] {
            assert!(contains_timestamp(timestamp));
        }
        for ordinary in [
            "x026-08-26",
            "2026x08-26",
            "2026-0x-26",
            "2026-08x26",
            "2026-08-2x",
            "x3:59:58",
            "23x59:58",
            "23:5x:58",
            "23:59x58",
            "23:59:5x",
        ] {
            assert!(!contains_timestamp(ordinary), "{ordinary}");
        }

        assert!(render_report(&PublicationReport::new("", "page")).is_err());
        assert!(render_report(&PublicationReport::new("check", "")).is_err());

        let existing = (0..40)
            .map(|index| format!("old-{index}"))
            .collect::<Vec<_>>()
            .join("\n");
        let candidate = (0..40)
            .map(|index| format!("new-{index}"))
            .collect::<Vec<_>>()
            .join("\n");
        assert!(
            render_parity_diff(Some(existing.as_bytes()), candidate.as_bytes())
                .contains("differing-lines-truncated")
        );

        for error in [
            ProjectionError::AmbiguousGraphLabel,
            ProjectionError::MissingSchemaIdentity,
            ProjectionError::MissingAtomRevision,
            ProjectionError::MissingFreshnessFacet,
            ProjectionError::MissingLogicalLocation,
            ProjectionError::MissingBoundary,
        ] {
            assert!(!error.stable_code().is_empty());
        }
        let valid_prefix = "Source graph: x\nbuoy/readable-trace-markdown/v1\nbuoy/trace-report/v1\nDerived data\nnavigation and impact only\nBuoy remains the current-state authority\n";
        assert_eq!(
            validate_markdown_projection(
                "buoy/readable-trace-markdown/v1\nbuoy/trace-report/v1\nDerived data\nnavigation and impact only\nBuoy remains the current-state authority\n"
            ),
            Err(ProjectionError::AmbiguousGraphLabel)
        );
        assert_eq!(
            validate_markdown_projection("Source graph: x\n- Graph : ambiguous\n"),
            Err(ProjectionError::AmbiguousGraphLabel)
        );
        assert_eq!(
            validate_markdown_projection("Source graph: x\nbuoy/readable-trace-markdown/v1\n"),
            Err(ProjectionError::MissingSchemaIdentity)
        );
        for boundary in [
            "Source graph: x\nbuoy/readable-trace-markdown/v1\nbuoy/trace-report/v1\n",
            "Source graph: x\nbuoy/readable-trace-markdown/v1\nbuoy/trace-report/v1\nDerived data\n",
            "Source graph: x\nbuoy/readable-trace-markdown/v1\nbuoy/trace-report/v1\nDerived data\nnavigation and impact only\n",
        ] {
            assert_eq!(
                validate_markdown_projection(boundary),
                Err(ProjectionError::MissingBoundary)
            );
        }
        assert_eq!(
            validate_markdown_projection(valid_prefix),
            Err(ProjectionError::MissingAtomRevision)
        );
        assert_eq!(
            validate_markdown_projection(&format!(
                "{valid_prefix}| State | Requirement path | Implementation path | Atom revision |\n"
            )),
            Err(ProjectionError::MissingFreshnessFacet)
        );
        assert_eq!(
            validate_markdown_projection(&format!(
                "{valid_prefix}| State | Implementation path | Atom revision | Freshness |\n"
            )),
            Err(ProjectionError::MissingLogicalLocation)
        );
        assert_eq!(
            validate_markdown_projection(&format!(
                "{valid_prefix}| State | Requirement path | Atom revision | Freshness |\n"
            )),
            Err(ProjectionError::MissingLogicalLocation)
        );
        let row_prefix = format!(
            "{valid_prefix}| State | Requirement path | Implementation path | Atom revision | Freshness |\n|---|---|---|---|---|\n"
        );
        assert_eq!(
            validate_markdown_projection(&format!(
                "{row_prefix}| current | req | impl | atom | record=true |\n"
            )),
            Err(ProjectionError::MissingFreshnessFacet)
        );

        assert!(
            validate_package_variant(&PackageVariant {
                features: Vec::new(),
                source_revision: "revision",
                ambient_cargo_home: None,
            })
            .is_err()
        );
        assert!(
            validate_package_variant(&PackageVariant {
                features: vec!["unknown"],
                source_revision: "revision",
                ambient_cargo_home: None,
            })
            .is_err()
        );
        assert!(
            parse_completion_evidence(&format!(
                "{EVIDENCE_BEGIN}\nPublication ID: <publication-id>\n{EVIDENCE_END}"
            ))
            .is_err()
        );
        assert!(
            parse_completion_evidence(&format!(
                "{EVIDENCE_BEGIN}\nPublication ID: publication\nRecord: TBD\n{EVIDENCE_END}"
            ))
            .is_err()
        );
        assert!(parse_completion_evidence("No further query is needed").is_err());
        assert!(is_placeholder_identity("<publication-id>"));
        assert!(!is_placeholder_identity("<publication-id"));
        assert!(!is_placeholder_identity("publication-id>"));
        let record = "xxh3-128:11111111111111111111111111111111";
        let matching = format!(
            "{EVIDENCE_BEGIN}\nPublication ID: publication\nRecord: {record}\n{EVIDENCE_END}\n| {record} |"
        );
        assert!(parse_completion_evidence(&matching).is_ok());
        let canonical_label = format!(
            "{EVIDENCE_BEGIN}\nPublication ID: publication\nRecord: {record}\n{EVIDENCE_END}\n| Canonical record locator | xxh3-128:22222222222222222222222222222222 |"
        );
        assert!(parse_completion_evidence(&canonical_label).is_ok());
        let conflict = format!(
            "{EVIDENCE_BEGIN}\nPublication ID: publication\nRecord: {record}\n{EVIDENCE_END}\n| other | sha-256:2222222222222222222222222222222222222222222222222222222222222222 |"
        );
        assert!(parse_completion_evidence(&conflict).is_err());
    }

    #[test]
    fn duplicate_guard_and_plan_parser_cover_owned_and_invalid_values() {
        let expected: &dyn serde::de::Expected = &DuplicateKeyGuardVisitor;
        assert_eq!(
            format!("{expected}"),
            "any JSON value with no duplicate object keys"
        );
        let owned: Result<DuplicateKeyGuard, serde::de::value::Error> =
            serde::de::Visitor::visit_string(DuplicateKeyGuardVisitor, String::from("owned"));
        assert!(owned.is_ok());

        let valid = include_bytes!("../fixtures/trace-publication/v1/plan-positive.json");
        let valid_text = core::str::from_utf8(valid).expect("valid plan text");
        drop(parse_plan(
            valid_text
                .replacen("\"expected_rows\":1", "\"expected_rows\":\"one\"", 1)
                .as_bytes(),
        ));
        drop(parse_plan(
            valid_text
                .replacen(
                    "\"publication_id\":\"demo-publication\"",
                    "\"publication_id\":\"\"",
                    1,
                )
                .as_bytes(),
        ));
        drop(parse_plan(br#"{"schema":"a","schema":"b"}"#));
    }

    #[test]
    fn basis_encoding_covers_every_selector_and_unavailable_member() {
        let row = complete_row();
        let snapshot = vec![(String::from("src/lib.rs"), b"source".to_vec())];
        for selector in [
            Selector::RequirementId(String::from("REQ-001")),
            Selector::RelationId(String::from("RELATION-1")),
            Selector::ImplementationPath(String::from("src/lib.rs")),
            Selector::Record(String::from("xxh3-128:record")),
        ] {
            assert!(
                compute_publication_basis(&BasisInputs {
                    plan_schema: PLAN_SCHEMA,
                    renderer_schema: "buoy/readable-trace-markdown/v1",
                    active_member: "xxh3-128",
                    selector: &selector,
                    trace_rows: core::slice::from_ref(&row),
                    normalized_snapshot: &snapshot,
                })
                .is_ok()
            );
        }
        assert_eq!(
            compute_publication_basis(&BasisInputs {
                plan_schema: PLAN_SCHEMA,
                renderer_schema: "buoy/readable-trace-markdown/v1",
                active_member: "unknown",
                selector: &Selector::RequirementId(String::from("REQ-001")),
                trace_rows: core::slice::from_ref(&row),
                normalized_snapshot: &snapshot,
            }),
            Err(PublicationError::DigestMemberUnavailable)
        );
    }
}
