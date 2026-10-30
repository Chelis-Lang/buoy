//! Offline, explicit-input migration from archived v1 material to active v3 bindings.

use buoy_core::{
    ActiveAlgorithm, ActiveAlgorithmError, CanonicalValue, DigestAlgorithm, SemanticDigest,
    semantic_digest_observation_with, semantic_digest_with,
};
use serde::Deserialize;
use std::collections::{BTreeMap, BTreeSet};

/// Stable fail-closed migration error.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum MigrationError {
    /// The migration document is malformed or has unsupported fields.
    Input,
    /// Source/target schema, encoding, or algorithm profile is not the frozen cutover.
    MixedVersion,
    /// A required authoritative byte sequence is absent or malformed.
    MissingAuthoritativeBytes,
    /// A required approval record is absent.
    MissingApproval,
    /// A nested binding is missing, duplicated, or cyclic.
    StaleNestedBinding,
    /// Migration changes or incompletely represents lifecycle/tombstone state.
    IllegalLifecycle,
    /// One or more changed identities lack an exact impact disposition.
    IncompleteImpact,
    /// Two leaf bindings have the same role and stable ID.
    DuplicateStableId,
    /// Canonical v3 encoding failed.
    Encoding,
    /// Unequal simultaneously available canonical sources produced one digest.
    ObservedCollision,
}

impl MigrationError {
    /// Stable machine diagnostic code.
    #[must_use]
    pub const fn code(self) -> &'static str {
        match self {
            Self::Input => "XXH-MIGRATION-INPUT",
            Self::MixedVersion => "XXH-MIGRATION-MIXED-VERSION",
            Self::MissingAuthoritativeBytes => "XXH-MIGRATION-MISSING-AUTHORITATIVE-BYTES",
            Self::MissingApproval => "XXH-MIGRATION-MISSING-APPROVAL",
            Self::StaleNestedBinding => "XXH-MIGRATION-STALE-NESTED-BINDING",
            Self::IllegalLifecycle => "XXH-MIGRATION-ILLEGAL-LIFECYCLE",
            Self::IncompleteImpact => "XXH-MIGRATION-INCOMPLETE-IMPACT",
            Self::DuplicateStableId => "XXH-MIGRATION-DUPLICATE-ID",
            Self::Encoding => "XXH-MIGRATION-ENCODING",
            Self::ObservedCollision => "DET-DIGEST-OBSERVED-COLLISION",
        }
    }
}

fn migration_active_result<T>(
    result: Result<T, ActiveAlgorithmError>,
) -> Result<T, MigrationError> {
    result.map_err(|_| MigrationError::MixedVersion)
}

/// Runs the migration active-algorithm error translation for coverage controls.
///
/// # Errors
///
/// Always returns the stable mixed-version error.
#[cfg(feature = "coverage-controls")]
#[doc(hidden)]
pub fn migration_active_error_control() -> Result<ActiveAlgorithm, MigrationError> {
    migration_active_result(Err(ActiveAlgorithmError::Unavailable(
        DigestAlgorithm::Xxh3_128,
    )))
}

/// Leaf binding role in deterministic migration order.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum LeafBindingRole {
    /// Repository execution configuration regenerated from complete saved bytes.
    Configuration,
    /// Semantic object's authoritative raw source artifact.
    ObjectAuthority,
    /// Required approval record's authoritative raw source artifact.
    ApprovalRecord,
}

impl LeafBindingRole {
    /// Stable report spelling.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Configuration => "configuration",
            Self::ObjectAuthority => "object-authority",
            Self::ApprovalRecord => "approval-record",
        }
    }
}

/// One regenerated v3 leaf binding.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MigratedLeafBinding {
    /// Deterministic role.
    pub role: LeafBindingRole,
    /// Stable source/configuration/approval identifier.
    pub stable_id: String,
    /// Canonical repository-relative source path.
    pub authoritative_path: String,
    /// Exact declared media type.
    pub media_type: String,
    /// Complete v3 digest regenerated from authoritative bytes.
    pub digest: SemanticDigest,
}

/// Complete deterministic leaf phase output.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LeafMigration {
    /// Frozen target semantic schema.
    pub schema: String,
    /// Totally ordered regenerated leaf bindings.
    pub bindings: Vec<MigratedLeafBinding>,
}

/// One dependency-closed semantic object regenerated under version 3.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MigratedObject {
    /// Deterministic dependency-order position.
    pub order: usize,
    /// Stable object ID preserved from version 1.
    pub stable_id: String,
    /// Closed semantic object/report kind.
    pub kind: String,
    /// Preserved lifecycle spelling.
    pub lifecycle: String,
    /// Regenerated v3 semantic digest.
    pub digest: SemanticDigest,
    /// Stable IDs of direct dependencies in canonical order.
    pub references: Vec<String>,
    /// Preserved tombstone replacement, when required.
    pub replacement: Option<String>,
}

/// Closed impact disposition for one changed identity.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum MigrationDisposition {
    /// Semantic content or binding was revised.
    Revised,
    /// Meaning was reviewed and reaffirmed under the regenerated binding.
    Reaffirmed,
    /// Terminal identity remains retired as a tombstone.
    Retired,
    /// An explicitly governed waiver covers the migration change.
    Waived,
}

impl MigrationDisposition {
    fn parse(value: &str) -> Option<Self> {
        match value {
            "revised" => Some(Self::Revised),
            "reaffirmed" => Some(Self::Reaffirmed),
            "retired" => Some(Self::Retired),
            "waived" => Some(Self::Waived),
            _ => None,
        }
    }

    /// Stable report spelling.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Revised => "revised",
            Self::Reaffirmed => "reaffirmed",
            Self::Retired => "retired",
            Self::Waived => "waived",
        }
    }
}

/// One old-to-new identity change with its reviewed disposition.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MigrationImpact {
    /// Stable identity whose binding changed.
    pub stable_id: String,
    /// Archived v1 digest text retained only as report data.
    pub old_digest: String,
    /// Regenerated v3 digest.
    pub new_digest: SemanticDigest,
    /// Required migration disposition.
    pub disposition: MigrationDisposition,
}

/// Complete leaf and dependency-ordered semantic migration output.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CompleteMigration {
    /// Frozen target semantic schema.
    pub schema: String,
    /// Regenerated leaf phase.
    pub leaves: Vec<MigratedLeafBinding>,
    /// Regenerated objects, graphs, and reports in dependency order.
    pub objects: Vec<MigratedObject>,
    /// Totally ordered complete impact closure.
    pub impacts: Vec<MigrationImpact>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct MigrationInput {
    schema: String,
    source_schema: String,
    target_schema: String,
    encoding_version: u16,
    target_encoding_version: u16,
    algorithm: String,
    target_algorithm: String,
    terminal_stable_ids: Vec<String>,
    #[serde(default)]
    configurations: Vec<ConfigurationInput>,
    objects: Vec<ObjectInput>,
    approvals: Vec<ApprovalInput>,
    impact_dispositions: Vec<ImpactInput>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct ConfigurationInput {
    id: String,
    schema_id: String,
    authoritative_path: String,
    authoritative_media_type: String,
    authoritative_bytes_hex: Option<String>,
    old_digest: String,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct ObjectInput {
    stable_id: String,
    kind: String,
    lifecycle: String,
    authoritative_path: String,
    authoritative_media_type: String,
    authoritative_bytes_hex: Option<String>,
    old_digest: String,
    approval: String,
    #[serde(default)]
    replacement: Option<String>,
    #[serde(default)]
    nested_bindings: Vec<String>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct ApprovalInput {
    id: String,
    authority: String,
    record_path: String,
    record_bytes_hex: Option<String>,
    old_digest: String,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct ImpactInput {
    stable_id: String,
    disposition: String,
}

#[allow(clippy::cast_possible_truncation)]
fn decode_hex(value: Option<&str>) -> Result<Vec<u8>, MigrationError> {
    let value = value.ok_or(MigrationError::MissingAuthoritativeBytes)?;
    if value.is_empty() || value.len() % 2 != 0 {
        return Err(MigrationError::MissingAuthoritativeBytes);
    }
    value
        .as_bytes()
        .chunks_exact(2)
        .map(|pair| {
            let high = char::from(pair[0])
                .to_digit(16)
                .ok_or(MigrationError::MissingAuthoritativeBytes)?;
            let low = char::from(pair[1])
                .to_digit(16)
                .ok_or(MigrationError::MissingAuthoritativeBytes)?;
            Ok(((high << 4) | low) as u8)
        })
        .collect()
}

fn raw_artifact_digest(
    active: ActiveAlgorithm,
    media_type: &str,
    bytes: &[u8],
) -> Result<SemanticDigest, MigrationError> {
    semantic_digest_with(
        active,
        "raw-artifact",
        vec![
            (1, CanonicalValue::Text(media_type.to_owned())),
            (2, CanonicalValue::Bytes(bytes.to_vec())),
        ],
    )
    .map_err(|_| MigrationError::Encoding)
}

fn configuration_digest(
    active: ActiveAlgorithm,
    schema_id: &str,
    media_type: &str,
    bytes: &[u8],
) -> Result<SemanticDigest, MigrationError> {
    semantic_digest_with(
        active,
        "repository-config",
        vec![
            (1, CanonicalValue::Text(schema_id.to_owned())),
            (2, CanonicalValue::Text(media_type.to_owned())),
            (3, CanonicalValue::Bytes(bytes.to_vec())),
        ],
    )
    .map_err(|_| MigrationError::Encoding)
}

fn observe(
    seen: &mut BTreeMap<SemanticDigest, Vec<u8>>,
    digest: SemanticDigest,
    canonical_source: Vec<u8>,
) -> Result<(), MigrationError> {
    if let Some(previous) = seen.get(&digest) {
        if previous != &canonical_source {
            return Err(MigrationError::ObservedCollision);
        }
    } else {
        seen.insert(digest, canonical_source);
    }
    Ok(())
}

/// Injects a reviewed digest collision into the migration observation guard.
///
/// # Errors
///
/// Always returns the reviewed collision diagnostic when the guard is active.
#[cfg(feature = "coverage-controls")]
pub fn observe_collision_control() -> Result<(), MigrationError> {
    let digest = SemanticDigest::xxh3_128([0; 16]);
    let mut seen = BTreeMap::from([(digest, vec![1])]);
    observe(&mut seen, digest, vec![2])
}

#[allow(clippy::expect_used)]
fn canonical_source(
    role: LeafBindingRole,
    schema: Option<&str>,
    media_type: &str,
    bytes: &[u8],
) -> Vec<u8> {
    let role_tag = match role {
        LeafBindingRole::Configuration => 1,
        LeafBindingRole::ObjectAuthority => 2,
        LeafBindingRole::ApprovalRecord => 3,
    };
    CanonicalValue::Record(vec![
        (1, CanonicalValue::U64(role_tag)),
        (
            2,
            schema.map_or(CanonicalValue::None, |value| {
                CanonicalValue::Some(Box::new(CanonicalValue::Text(value.to_owned())))
            }),
        ),
        (3, CanonicalValue::Text(media_type.to_owned())),
        (4, CanonicalValue::Bytes(bytes.to_vec())),
    ])
    .encode()
    .expect("bounded migration observation fields canonically encode")
}

#[derive(Clone, Copy)]
enum ArchivalDigestFormat {
    Sha256,
    Xxh3_128,
}

fn validate_archival_digest(
    value: &str,
    format: ArchivalDigestFormat,
) -> Result<(), MigrationError> {
    let (prefix, length) = match format {
        ArchivalDigestFormat::Sha256 => ("sha256:", 64),
        ArchivalDigestFormat::Xxh3_128 => ("xxh3-128:", 32),
    };
    let payload = value.strip_prefix(prefix).ok_or(MigrationError::Input)?;
    if payload.len() != length
        || !payload
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
    {
        return Err(MigrationError::Input);
    }
    Ok(())
}

/// Regenerates leaf raw-artifact, approval-record, and configuration bindings offline.
///
/// Old digest text is parsed only as archival migration input shape and is never
/// transformed, truncated, padded, relabeled, or supplied to the v3 registry.
///
/// # Errors
///
/// Fails closed on malformed input/profile, absent authoritative bytes,
/// duplicate leaf IDs, canonical encoding failure, or an observed collision.
#[allow(clippy::too_many_lines)]
pub fn migrate_leaf_bindings(source: &str) -> Result<LeafMigration, MigrationError> {
    migrate_leaf_bindings_internal(source, None, false)
}

/// Injects one role-owned collision through the complete leaf migration path.
///
/// # Errors
///
/// Returns leaf migration validation or injected collision failures.
#[cfg(feature = "coverage-controls")]
pub fn migrate_leaf_collision_control(
    source: &str,
    role: LeafBindingRole,
) -> Result<LeafMigration, MigrationError> {
    migrate_leaf_bindings_internal(source, Some(role), false)
}

/// Injects an unavailable target adapter through the complete leaf migration path.
///
/// # Errors
///
/// Always returns the stable mixed-version error for valid migration input.
#[cfg(feature = "coverage-controls")]
pub fn migrate_leaf_unavailable_control(source: &str) -> Result<LeafMigration, MigrationError> {
    migrate_leaf_bindings_internal(source, None, true)
}

#[allow(clippy::expect_used, clippy::too_many_lines)]
fn migrate_leaf_bindings_internal(
    source: &str,
    collision_role: Option<LeafBindingRole>,
    force_unavailable: bool,
) -> Result<LeafMigration, MigrationError> {
    let input = toml::from_str::<MigrationInput>(source).map_err(|_| MigrationError::Input)?;
    if input.schema != "spec-provenance-migration-input/v1"
        || input.target_schema != "spec-provenance/v3"
        || input.target_encoding_version != 3
    {
        return Err(MigrationError::MixedVersion);
    }
    let archival_format = match (
        input.source_schema.as_str(),
        input.encoding_version,
        input.algorithm.as_str(),
    ) {
        ("spec-provenance/v1", 1, "sha256") => ArchivalDigestFormat::Sha256,
        ("spec-provenance/v2", 2, "xxh3-128") => ArchivalDigestFormat::Xxh3_128,
        _ => return Err(MigrationError::MixedVersion),
    };
    let target_algorithm = DigestAlgorithm::from_canonical_name(&input.target_algorithm)
        .ok_or(MigrationError::MixedVersion)?;
    let active_result = if force_unavailable {
        Err(ActiveAlgorithmError::Unavailable(target_algorithm))
    } else {
        ActiveAlgorithm::new(target_algorithm)
    };
    let active = migration_active_result(active_result)?;
    let terminal_ids = input.terminal_stable_ids.iter().collect::<BTreeSet<_>>();
    if terminal_ids.len() != input.terminal_stable_ids.len()
        || terminal_ids.iter().any(|stable_id| stable_id.is_empty())
    {
        return Err(MigrationError::Input);
    }

    let mut bindings = Vec::new();
    let mut keys = BTreeSet::new();
    let mut observations = BTreeMap::new();

    for configuration in input.configurations {
        validate_archival_digest(&configuration.old_digest, archival_format)?;
        if configuration.id.is_empty()
            || configuration.schema_id.is_empty()
            || configuration.authoritative_path.is_empty()
            || configuration.authoritative_media_type.is_empty()
        {
            return Err(MigrationError::Input);
        }
        let key = (LeafBindingRole::Configuration, configuration.id.clone());
        if !keys.insert(key) {
            return Err(MigrationError::DuplicateStableId);
        }
        let bytes = decode_hex(configuration.authoritative_bytes_hex.as_deref())?;
        let digest = configuration_digest(
            active,
            &configuration.schema_id,
            &configuration.authoritative_media_type,
            &bytes,
        )?;
        if collision_role == Some(LeafBindingRole::Configuration) {
            observations.insert(digest, vec![0xff]);
        }
        observe(
            &mut observations,
            digest,
            canonical_source(
                LeafBindingRole::Configuration,
                Some(&configuration.schema_id),
                &configuration.authoritative_media_type,
                &bytes,
            ),
        )?;
        bindings.push(MigratedLeafBinding {
            role: LeafBindingRole::Configuration,
            stable_id: configuration.id,
            authoritative_path: configuration.authoritative_path,
            media_type: configuration.authoritative_media_type,
            digest,
        });
    }

    for object in input.objects {
        validate_archival_digest(&object.old_digest, archival_format)?;
        if object.stable_id.is_empty()
            || object.kind.is_empty()
            || object.lifecycle.is_empty()
            || object.authoritative_path.is_empty()
            || object.authoritative_media_type.is_empty()
            || object.approval.is_empty()
            || object.replacement.as_deref() == Some("")
            || object.nested_bindings.iter().any(String::is_empty)
        {
            return Err(MigrationError::Input);
        }
        let key = (LeafBindingRole::ObjectAuthority, object.stable_id.clone());
        if !keys.insert(key) {
            return Err(MigrationError::DuplicateStableId);
        }
        let bytes = decode_hex(object.authoritative_bytes_hex.as_deref())?;
        let digest = raw_artifact_digest(active, &object.authoritative_media_type, &bytes)?;
        if collision_role == Some(LeafBindingRole::ObjectAuthority) {
            observations.insert(digest, vec![0xff]);
        }
        observe(
            &mut observations,
            digest,
            canonical_source(
                LeafBindingRole::ObjectAuthority,
                None,
                &object.authoritative_media_type,
                &bytes,
            ),
        )?;
        bindings.push(MigratedLeafBinding {
            role: LeafBindingRole::ObjectAuthority,
            stable_id: object.stable_id,
            authoritative_path: object.authoritative_path,
            media_type: object.authoritative_media_type,
            digest,
        });
    }

    for approval in input.approvals {
        validate_archival_digest(&approval.old_digest, archival_format)?;
        if approval.id.is_empty()
            || approval.authority.is_empty()
            || approval.record_path.is_empty()
        {
            return Err(MigrationError::Input);
        }
        let key = (LeafBindingRole::ApprovalRecord, approval.id.clone());
        if !keys.insert(key) {
            return Err(MigrationError::DuplicateStableId);
        }
        let bytes = decode_hex(approval.record_bytes_hex.as_deref())?;
        let digest = buoy_core::raw_artifact_digest_with(
            active,
            buoy_core::RawArtifactMediaType::Markdown,
            &bytes,
        );
        if collision_role == Some(LeafBindingRole::ApprovalRecord) {
            observations.insert(digest, vec![0xff]);
        }
        observe(
            &mut observations,
            digest,
            canonical_source(
                LeafBindingRole::ApprovalRecord,
                None,
                "text/markdown",
                &bytes,
            ),
        )?;
        bindings.push(MigratedLeafBinding {
            role: LeafBindingRole::ApprovalRecord,
            stable_id: approval.id,
            authoritative_path: approval.record_path,
            media_type: String::from("text/markdown"),
            digest,
        });
    }

    for impact in input.impact_dispositions {
        if impact.stable_id.is_empty() || impact.disposition.is_empty() {
            return Err(MigrationError::Input);
        }
    }
    bindings.sort_unstable_by(|left, right| {
        (left.role, &left.stable_id).cmp(&(right.role, &right.stable_id))
    });
    Ok(LeafMigration {
        schema: input.target_schema,
        bindings,
    })
}

fn object_kind(kind: &str) -> Option<(u8, &'static str)> {
    match kind {
        "atom" => Some((0, "atom-object")),
        "tombstone" => Some((0, "tombstone")),
        "evidence-carrier" => Some((1, "evidence-carrier")),
        "coverage-policy" => Some((1, "coverage-policy-object")),
        "derivation" => Some((1, "derivation")),
        "assumption" => Some((1, "assumption")),
        "formal-property" => Some((1, "formal-property")),
        "proof-record" => Some((2, "proof-record")),
        "spec-graph" => Some((3, "spec-graph-complete")),
        "static-report" => Some((4, "static-report")),
        "impact-report" => Some((4, "impact-report")),
        "execution-report" => Some((5, "execution-report")),
        _ => None,
    }
}

fn validate_lifecycle(object: &ObjectInput) -> Result<(), MigrationError> {
    let terminal = matches!(object.lifecycle.as_str(), "superseded" | "withdrawn");
    let live = matches!(object.lifecycle.as_str(), "proposed" | "ratified");
    if ((object.kind == "tombstone") & ((!terminal) | object.replacement.is_none()))
        | ((object.kind != "tombstone") & ((!live) | object.replacement.is_some()))
    {
        return Err(MigrationError::IllegalLifecycle);
    }
    Ok(())
}

fn object_dependencies(object: &ObjectInput) -> Vec<&str> {
    object
        .nested_bindings
        .iter()
        .map(String::as_str)
        .chain(object.replacement.iter().map(String::as_str))
        .collect()
}

#[allow(clippy::expect_used)]
fn semantic_object_fields(
    object: &ObjectInput,
    source_digest: SemanticDigest,
    approval_digest: SemanticDigest,
    migrated: &BTreeMap<String, SemanticDigest>,
) -> Result<Vec<(u16, CanonicalValue)>, MigrationError> {
    let mut reference_ids = object.nested_bindings.clone();
    reference_ids.sort_unstable();
    if reference_ids.windows(2).any(|pair| pair[0] == pair[1]) {
        return Err(MigrationError::StaleNestedBinding);
    }
    let references = reference_ids
        .iter()
        .map(|stable_id| {
            let digest = migrated
                .get(stable_id)
                .copied()
                .expect("dependency scheduling migrated every referenced object");
            CanonicalValue::Record(vec![
                (1, CanonicalValue::Text(stable_id.clone())),
                (2, CanonicalValue::Digest(digest)),
            ])
        })
        .collect::<Vec<_>>();
    let replacement = object
        .replacement
        .as_ref()
        .map_or(CanonicalValue::None, |stable_id| {
            let digest = migrated
                .get(stable_id)
                .copied()
                .expect("dependency scheduling migrated the replacement object");
            CanonicalValue::Some(Box::new(CanonicalValue::Record(vec![
                (1, CanonicalValue::Text(stable_id.clone())),
                (2, CanonicalValue::Digest(digest)),
            ])))
        });
    Ok(vec![
        (1, CanonicalValue::Text(object.kind.clone())),
        (2, CanonicalValue::Text(object.stable_id.clone())),
        (3, CanonicalValue::Text(object.lifecycle.clone())),
        (4, CanonicalValue::Digest(source_digest)),
        (5, CanonicalValue::Digest(approval_digest)),
        (6, CanonicalValue::Set(references)),
        (7, replacement),
    ])
}

/// Regenerates every semantic object, nested reference, graph, and report in
/// deterministic dependency order after completing the leaf phase.
///
/// # Errors
///
/// Fails closed on every leaf error, unknown object kind, missing approval,
/// stale/duplicate/cyclic nested binding, encoding failure, or observed collision.
#[allow(clippy::too_many_lines)]
pub fn migrate_complete(source: &str) -> Result<CompleteMigration, MigrationError> {
    migrate_complete_internal(source, false)
}

/// Injects one semantic-object collision through complete migration propagation.
///
/// # Errors
///
/// Returns complete migration validation or injected collision failures.
#[cfg(feature = "coverage-controls")]
pub fn migrate_complete_collision_control(
    source: &str,
) -> Result<CompleteMigration, MigrationError> {
    migrate_complete_internal(source, true)
}

#[allow(
    clippy::expect_used,
    clippy::needless_bitwise_bool,
    clippy::too_many_lines
)]
fn migrate_complete_internal(
    source: &str,
    inject_object_collision: bool,
) -> Result<CompleteMigration, MigrationError> {
    let leaf = migrate_leaf_bindings(source)?;
    let input = toml::from_str::<MigrationInput>(source)
        .expect("leaf migration already parsed this immutable input");
    let algorithm = DigestAlgorithm::from_canonical_name(&input.target_algorithm)
        .expect("leaf migration validated the target algorithm");
    let active = ActiveAlgorithm::new(algorithm)
        .expect("leaf migration validated target algorithm availability");
    let mut old_digests = BTreeMap::new();
    for (stable_id, old_digest) in input
        .configurations
        .iter()
        .map(|value| (&value.id, &value.old_digest))
        .chain(
            input
                .objects
                .iter()
                .map(|value| (&value.stable_id, &value.old_digest)),
        )
        .chain(
            input
                .approvals
                .iter()
                .map(|value| (&value.id, &value.old_digest)),
        )
    {
        if old_digests
            .insert(stable_id.clone(), old_digest.clone())
            .is_some()
        {
            return Err(MigrationError::DuplicateStableId);
        }
    }
    let object_sources = leaf
        .bindings
        .iter()
        .filter(|binding| binding.role == LeafBindingRole::ObjectAuthority)
        .map(|binding| (binding.stable_id.clone(), binding.digest))
        .collect::<BTreeMap<_, _>>();
    let approvals = leaf
        .bindings
        .iter()
        .filter(|binding| binding.role == LeafBindingRole::ApprovalRecord)
        .map(|binding| (binding.stable_id.clone(), binding.digest))
        .collect::<BTreeMap<_, _>>();
    let expected_terminal_ids = input
        .terminal_stable_ids
        .iter()
        .cloned()
        .collect::<BTreeSet<_>>();
    let mut remaining = BTreeMap::new();
    for object in input.objects {
        if object_kind(&object.kind).is_none() {
            return Err(MigrationError::Input);
        }
        validate_lifecycle(&object)?;
        remaining.insert(object.stable_id.clone(), object);
    }
    let actual_terminal_ids = remaining
        .values()
        .filter(|object| object.kind == "tombstone")
        .map(|object| object.stable_id.clone())
        .collect::<BTreeSet<_>>();
    if actual_terminal_ids != expected_terminal_ids {
        return Err(MigrationError::IllegalLifecycle);
    }

    let mut migrated = BTreeMap::new();
    let mut objects = Vec::new();
    let mut observations = leaf
        .bindings
        .iter()
        .map(|binding| {
            (
                binding.digest,
                format!("leaf:{}:{}", binding.role.as_str(), binding.stable_id).into_bytes(),
            )
        })
        .collect::<BTreeMap<_, _>>();
    while !remaining.is_empty() {
        let next = remaining
            .iter()
            .filter(|(_, object)| {
                object_dependencies(object)
                    .iter()
                    .all(|reference| migrated.contains_key(*reference))
            })
            .min_by(|(left_id, left), (right_id, right)| {
                let left_phase = object_kind(&left.kind).map_or(u8::MAX, |value| value.0);
                let right_phase = object_kind(&right.kind).map_or(u8::MAX, |value| value.0);
                (left_phase, left_id.as_str()).cmp(&(right_phase, right_id.as_str()))
            })
            .map(|(stable_id, _)| stable_id.clone())
            .ok_or(MigrationError::StaleNestedBinding)?;
        let object = remaining
            .remove(&next)
            .expect("selected migration object remains present");
        let (_, domain) = object_kind(&object.kind).expect("object kinds were validated");
        let source_digest = object_sources
            .get(&object.stable_id)
            .copied()
            .expect("leaf migration emits every object authority binding");
        let approval_digest = approvals
            .get(&object.approval)
            .copied()
            .ok_or(MigrationError::MissingApproval)?;
        let fields = semantic_object_fields(&object, source_digest, approval_digest, &migrated)?;
        let digest_observation = semantic_digest_observation_with(active, domain, fields)
            .map_err(|_| MigrationError::Encoding)?;
        let digest = digest_observation.digest();
        if inject_object_collision {
            observations.insert(digest, vec![0xff]);
        }
        observe(
            &mut observations,
            digest,
            digest_observation.canonical_preimage().to_vec(),
        )?;
        let mut references = object.nested_bindings;
        references.sort_unstable();
        let order = objects.len();
        migrated.insert(object.stable_id.clone(), digest);
        objects.push(MigratedObject {
            order,
            stable_id: object.stable_id,
            kind: object.kind,
            lifecycle: object.lifecycle,
            digest,
            references,
            replacement: object.replacement,
        });
    }
    let mut new_digests = leaf
        .bindings
        .iter()
        .filter(|binding| binding.role != LeafBindingRole::ObjectAuthority)
        .map(|binding| (binding.stable_id.clone(), binding.digest))
        .collect::<BTreeMap<_, _>>();
    for object in &objects {
        new_digests.insert(object.stable_id.clone(), object.digest);
    }
    let mut dispositions = BTreeMap::new();
    for impact in input.impact_dispositions {
        let disposition = MigrationDisposition::parse(&impact.disposition)
            .ok_or(MigrationError::IncompleteImpact)?;
        if dispositions.insert(impact.stable_id, disposition).is_some() {
            return Err(MigrationError::IncompleteImpact);
        }
    }
    if !new_digests.keys().eq(dispositions.keys()) | !new_digests.keys().eq(old_digests.keys()) {
        return Err(MigrationError::IncompleteImpact);
    }
    let mut impacts = Vec::with_capacity(new_digests.len());
    for (stable_id, new_digest) in new_digests {
        let disposition = dispositions
            .get(&stable_id)
            .copied()
            .expect("impact keys were proven complete");
        let terminal = objects.iter().any(|object| {
            object.stable_id == stable_id
                && object.kind == "tombstone"
                && matches!(object.lifecycle.as_str(), "superseded" | "withdrawn")
        });
        if terminal != (disposition == MigrationDisposition::Retired) {
            return Err(MigrationError::IllegalLifecycle);
        }
        impacts.push(MigrationImpact {
            old_digest: old_digests
                .remove(&stable_id)
                .expect("archival digest keys were proven complete"),
            stable_id,
            new_digest,
            disposition,
        });
    }
    Ok(CompleteMigration {
        schema: leaf.schema,
        leaves: leaf.bindings,
        objects,
        impacts,
    })
}
