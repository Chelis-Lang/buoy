//! Fail-closed adversarial campaign bindings and portable platform corpus generation.

use crate::adapters::{AuthorityExpectation, extract_markdown, extract_rust};
use crate::glob::PortableGlob;
use crate::repository::RepositoryConfig;
use buoy_core::{
    AggregationInput, AssuranceClass, CanonicalStaticReport, CanonicalValue, DecodeBudget,
    RepositoryPath, SemanticDigest, Verdict, aggregate_assurance, semantic_digest,
};
use serde::Deserialize;
use std::collections::BTreeSet;
use std::fmt::Write as _;
use std::fs;
use std::io;
use std::path::Path;

const FUZZ_POLICY: &str = "assurance/adversarial/v1/fuzz-policy.toml";
const FUZZ_ARTIFACT: &str = "assurance/adversarial/v1/fuzz-campaign.toml";
const MUTATION_POLICY: &str = "assurance/adversarial/v1/mutation-policy.toml";
const MUTATION_ARTIFACT: &str = "assurance/adversarial/v1/mutation-campaign.toml";
const NAMED_MUTANTS: &str = "assurance/adversarial/v1/named-mutants.tsv";
const MAX_BOUND_FILES: usize = 20_000;
const MAX_BOUND_BYTES: u64 = 268_435_456;

/// Closed campaign class. Fuzz and mutation evidence never satisfy one another.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CampaignKind {
    /// Coverage-guided fuzz campaign.
    Fuzz,
    /// Automated source-mutation campaign.
    Mutation,
}

impl CampaignKind {
    const fn as_str(self) -> &'static str {
        match self {
            Self::Fuzz => "fuzz",
            Self::Mutation => "mutation",
        }
    }

    const fn paths(self) -> (&'static str, &'static str) {
        match self {
            Self::Fuzz => (FUZZ_POLICY, FUZZ_ARTIFACT),
            Self::Mutation => (MUTATION_POLICY, MUTATION_ARTIFACT),
        }
    }

    const fn tool(self) -> (&'static str, &'static str, &'static str) {
        match self {
            Self::Fuzz => (
                "cargo-fuzz",
                "0.13.2",
                "assurance/v1/toolchain.toml#cargo-fuzz@0.13.2",
            ),
            Self::Mutation => (
                "cargo-mutants",
                "27.1.0",
                "assurance/v1/toolchain.toml#cargo-mutants@27.1.0",
            ),
        }
    }
}

/// Current deterministic source and configuration identities for one campaign.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CampaignBindings {
    /// Digest of every declared production/harness/test source path.
    pub source_digest: String,
    /// Digest of every declared policy, corpus, and tool-configuration path.
    pub configuration_digest: String,
}

/// Stable campaign validation failure.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CampaignError {
    /// Campaign policy could not be read or parsed.
    PolicySyntax,
    /// Campaign artifact could not be read or parsed.
    ArtifactSyntax,
    /// Campaign policy has an unknown or inconsistent closed field value.
    PolicySchema,
    /// Campaign artifact disagrees with its selected policy.
    ArtifactSchema,
    /// A declared bound input is missing, link-like, oversized, or unreadable.
    BoundInput,
    /// Artifact source identity differs from current declared source inputs.
    SourceStale,
    /// Artifact configuration identity differs from current declared configuration inputs.
    ConfigurationStale,
    /// Tool name, version, or repository-owned pin differs.
    ToolBinding,
    /// Artifact is not yet valid, expired, or exceeds the policy age bound.
    Expired,
    /// Campaign did not complete with zero unresolved blocking findings.
    CampaignFailed,
    /// Required target/mutation units are missing, duplicated, or unknown.
    UnitSet,
    /// A required campaign unit did not pass or retained findings.
    UnitFailed,
    /// Cases, duration, score fields, or aggregate counts violate policy.
    Bounds,
    /// A non-equivalent accepted mutant survived or the score is below policy.
    MutationScore,
    /// Equivalent/unsupported survivor lacks an exact reviewed nonpassing disposition.
    SurvivorDisposition,
    /// A discharge attempted to count stale or missing campaign evidence as satisfied.
    DischargeCannotSatisfy,
    /// Named high-consequence mutant catalog is malformed or incomplete.
    NamedMutants,
    /// Requested platform is outside the declared matrix.
    Platform,
    /// Generic portable corpus row construction failed.
    Corpus,
    /// Canonical-value corpus construction failed.
    CorpusCanonical,
    /// Path/glob corpus construction failed.
    CorpusPath,
    /// Configuration/manifest corpus construction failed.
    CorpusConfiguration,
    /// Governance corpus construction failed.
    CorpusGovernance,
    /// Canonical-report corpus construction failed.
    CorpusReport,
}

impl CampaignError {
    /// Stable fail-closed diagnostic code.
    #[must_use]
    pub const fn code(self) -> &'static str {
        match self {
            Self::PolicySyntax => "ADV-CAMPAIGN-POLICY-SYNTAX",
            Self::ArtifactSyntax => "ADV-CAMPAIGN-ARTIFACT-SYNTAX",
            Self::PolicySchema => "ADV-CAMPAIGN-POLICY-SCHEMA",
            Self::ArtifactSchema => "ADV-CAMPAIGN-ARTIFACT-SCHEMA",
            Self::BoundInput => "ADV-CAMPAIGN-BOUND-INPUT",
            Self::SourceStale => "ADV-CAMPAIGN-SOURCE-STALE",
            Self::ConfigurationStale => "ADV-CAMPAIGN-CONFIGURATION-STALE",
            Self::ToolBinding => "ADV-CAMPAIGN-TOOL-STALE",
            Self::Expired => "ADV-CAMPAIGN-EXPIRED",
            Self::CampaignFailed => "ADV-CAMPAIGN-FAILED",
            Self::UnitSet => "ADV-CAMPAIGN-UNIT-SET",
            Self::UnitFailed => "ADV-CAMPAIGN-UNIT-FAILED",
            Self::Bounds => "ADV-CAMPAIGN-BOUNDS",
            Self::MutationScore => "ADV-MUTATION-SCORE",
            Self::SurvivorDisposition => "ADV-MUTATION-SURVIVOR-DISPOSITION",
            Self::DischargeCannotSatisfy => "ADV-CAMPAIGN-DISCHARGE-NONPASSING",
            Self::NamedMutants => "ADV-MUTATION-NAMED-CATALOG",
            Self::Platform => "ADV-PLATFORM-UNKNOWN",
            Self::Corpus => "ADV-PLATFORM-CORPUS",
            Self::CorpusCanonical => "ADV-PLATFORM-CORPUS-CANONICAL",
            Self::CorpusPath => "ADV-PLATFORM-CORPUS-PATH",
            Self::CorpusConfiguration => "ADV-PLATFORM-CORPUS-CONFIGURATION",
            Self::CorpusGovernance => "ADV-PLATFORM-CORPUS-GOVERNANCE",
            Self::CorpusReport => "ADV-PLATFORM-CORPUS-REPORT",
        }
    }
}

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct CampaignPolicy {
    schema: String,
    id: String,
    kind: String,
    tool_id: String,
    tool_version: String,
    tool_binding: String,
    source_paths: Vec<String>,
    configuration_paths: Vec<String>,
    required_units: Vec<String>,
    minimum_cases: u64,
    minimum_duration_seconds: u64,
    minimum_score_basis_points: u64,
    maximum_age_days: u64,
    campaign_command: String,
    replay_command: String,
    assurance_scope: String,
    exclusions: Vec<String>,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct CampaignArtifact {
    schema: String,
    campaign_id: String,
    kind: String,
    tool_id: String,
    tool_version: String,
    tool_binding: String,
    source_digest: String,
    configuration_digest: String,
    generated_on: String,
    expires_on: String,
    command: String,
    result: String,
    total_cases: u64,
    duration_seconds: u64,
    accepted_mutants: u64,
    killed_mutants: u64,
    score_basis_points: u64,
    finding_count: u64,
    unresolved_blocking_findings: u64,
    assurance_scope: String,
    units: Vec<CampaignUnit>,
    #[serde(default)]
    survivors: Vec<CampaignSurvivor>,
    #[serde(default)]
    discharges: Vec<CampaignDischarge>,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct CampaignUnit {
    id: String,
    cases: u64,
    status: String,
    finding_count: u64,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct CampaignSurvivor {
    id: String,
    disposition: String,
    rationale: String,
    review: String,
    counts_as_killed: bool,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct CampaignDischarge {
    id: String,
    scope: String,
    rationale: String,
    review: String,
    counts_as_satisfied: bool,
}

/// Computes current campaign bindings from policy-declared paths.
///
/// # Errors
///
/// Fails closed for malformed policy or any missing, link-like, oversized, or
/// unreadable bound input.
pub fn campaign_bindings(
    root: &Path,
    kind: CampaignKind,
) -> Result<CampaignBindings, CampaignError> {
    let (policy_path, _) = kind.paths();
    let text =
        fs::read_to_string(root.join(policy_path)).map_err(|_| CampaignError::PolicySyntax)?;
    let policy = parse_policy(&text, kind)?;
    campaign_bindings_for_policy(root, &policy)
}

fn campaign_bindings_for_policy(
    root: &Path,
    policy: &CampaignPolicy,
) -> Result<CampaignBindings, CampaignError> {
    Ok(CampaignBindings {
        source_digest: digest_declared_paths(root, &policy.source_paths)?.canonical_text(),
        configuration_digest: digest_declared_paths(root, &policy.configuration_paths)?
            .canonical_text(),
    })
}

/// Validates one committed repository campaign against current bound inputs.
///
/// # Errors
///
/// Returns a stable fail-closed category for malformed, stale, expired,
/// discharged, incomplete, or failing evidence.
pub fn validate_repository_campaign(
    root: &Path,
    kind: CampaignKind,
    as_of: &str,
) -> Result<(), CampaignError> {
    let (policy_path, artifact_path) = kind.paths();
    let policy =
        fs::read_to_string(root.join(policy_path)).map_err(|_| CampaignError::PolicySyntax)?;
    let artifact =
        fs::read_to_string(root.join(artifact_path)).map_err(|_| CampaignError::ArtifactSyntax)?;
    let parsed_policy = parse_policy(&policy, kind)?;
    let bindings = campaign_bindings_for_policy(root, &parsed_policy)?;
    validate_campaign_documents(
        &policy,
        &artifact,
        &bindings.source_digest,
        &bindings.configuration_digest,
        as_of,
    )?;
    match kind {
        CampaignKind::Fuzz => validate_fuzz_layout(root, &parsed_policy),
        CampaignKind::Mutation => {
            validate_named_mutant_catalog(root)?;
            validate_mutation_layout(root)
        }
    }
}

/// Validates already-loaded policy and artifact documents against exact bindings.
///
/// # Errors
///
/// Returns the exact stale, expiry, discharge, result, unit, survivor, or score
/// category without consulting ambient repository state.
pub fn validate_campaign_documents(
    policy_text: &str,
    artifact_text: &str,
    expected_source_digest: &str,
    expected_configuration_digest: &str,
    as_of: &str,
) -> Result<(), CampaignError> {
    let policy_probe =
        toml::from_str::<CampaignPolicy>(policy_text).map_err(|_| CampaignError::PolicySyntax)?;
    let kind = match policy_probe.kind.as_str() {
        "fuzz" => CampaignKind::Fuzz,
        "mutation" => CampaignKind::Mutation,
        _ => return Err(CampaignError::PolicySchema),
    };
    let policy = parse_policy(policy_text, kind)?;
    let artifact = toml::from_str::<CampaignArtifact>(artifact_text)
        .map_err(|_| CampaignError::ArtifactSyntax)?;
    validate_artifact(
        &policy,
        &artifact,
        kind,
        expected_source_digest,
        expected_configuration_digest,
        as_of,
    )
}

fn parse_policy(text: &str, kind: CampaignKind) -> Result<CampaignPolicy, CampaignError> {
    let policy = toml::from_str::<CampaignPolicy>(text).map_err(|_| CampaignError::PolicySyntax)?;
    let (tool_id, tool_version, tool_binding) = kind.tool();
    if policy.schema != "buoy-adversarial-campaign-policy/v1"
        || policy.kind != kind.as_str()
        || policy.id.is_empty()
        || policy.tool_id != tool_id
        || policy.tool_version != tool_version
        || policy.tool_binding != tool_binding
        || policy.source_paths.is_empty()
        || policy.configuration_paths.is_empty()
        || policy.required_units.is_empty()
        || policy.minimum_cases == 0
        || policy.minimum_duration_seconds == 0
        || policy.maximum_age_days == 0
        || policy.maximum_age_days > 90
        || policy.campaign_command.is_empty()
        || policy.replay_command.is_empty()
        || policy.assurance_scope.is_empty()
        || has_duplicates(&policy.source_paths)
        || has_duplicates(&policy.configuration_paths)
        || has_duplicates(&policy.required_units)
        || has_duplicates(&policy.exclusions)
        || (kind == CampaignKind::Fuzz && policy.minimum_score_basis_points != 0)
        || (kind == CampaignKind::Mutation
            && !(1..=10_000).contains(&policy.minimum_score_basis_points))
    {
        return Err(CampaignError::PolicySchema);
    }
    Ok(policy)
}

#[allow(clippy::too_many_arguments, clippy::too_many_lines)]
fn validate_artifact(
    policy: &CampaignPolicy,
    artifact: &CampaignArtifact,
    kind: CampaignKind,
    expected_source_digest: &str,
    expected_configuration_digest: &str,
    as_of: &str,
) -> Result<(), CampaignError> {
    if artifact.schema != "buoy-adversarial-campaign-artifact/v1"
        || artifact.campaign_id != policy.id
        || artifact.kind != policy.kind
        || artifact.command != policy.campaign_command
        || artifact.assurance_scope != policy.assurance_scope
    {
        return Err(CampaignError::ArtifactSchema);
    }
    if artifact.tool_id != policy.tool_id
        || artifact.tool_version != policy.tool_version
        || artifact.tool_binding != policy.tool_binding
    {
        return Err(CampaignError::ToolBinding);
    }
    if SemanticDigest::parse_canonical(&artifact.source_digest).is_err()
        || artifact.source_digest != expected_source_digest
    {
        return Err(CampaignError::SourceStale);
    }
    if SemanticDigest::parse_canonical(&artifact.configuration_digest).is_err()
        || artifact.configuration_digest != expected_configuration_digest
    {
        return Err(CampaignError::ConfigurationStale);
    }
    validate_dates(
        &artifact.generated_on,
        &artifact.expires_on,
        as_of,
        policy.maximum_age_days,
    )?;
    if artifact.result != "pass"
        || artifact.finding_count != 0
        || artifact.unresolved_blocking_findings != 0
    {
        return Err(CampaignError::CampaignFailed);
    }
    if artifact.duration_seconds < policy.minimum_duration_seconds {
        return Err(CampaignError::Bounds);
    }
    let required = policy.required_units.iter().collect::<BTreeSet<_>>();
    let actual = artifact
        .units
        .iter()
        .map(|unit| &unit.id)
        .collect::<BTreeSet<_>>();
    if required != actual || actual.len() != artifact.units.len() {
        return Err(CampaignError::UnitSet);
    }
    if artifact
        .units
        .iter()
        .any(|unit| unit.status != "pass" || unit.finding_count != 0 || unit.cases == 0)
    {
        return Err(CampaignError::UnitFailed);
    }
    let cases = artifact
        .units
        .iter()
        .try_fold(0_u64, |total, unit| total.checked_add(unit.cases))
        .ok_or(CampaignError::Bounds)?;
    if cases != artifact.total_cases || cases < policy.minimum_cases {
        return Err(CampaignError::Bounds);
    }
    if artifact.discharges.iter().any(|discharge| {
        discharge.counts_as_satisfied
            || discharge.id.is_empty()
            || discharge.scope.is_empty()
            || discharge.rationale.is_empty()
            || discharge.review.is_empty()
    }) {
        return Err(CampaignError::DischargeCannotSatisfy);
    }
    if artifact.survivors.iter().any(|survivor| {
        !matches!(survivor.disposition.as_str(), "equivalent" | "unsupported")
            || survivor.id.is_empty()
            || survivor.rationale.is_empty()
            || survivor.review.is_empty()
            || survivor.counts_as_killed
    }) {
        return Err(CampaignError::SurvivorDisposition);
    }
    match kind {
        CampaignKind::Fuzz => {
            if artifact.accepted_mutants != 0
                || artifact.killed_mutants != 0
                || artifact.score_basis_points != 0
                || !artifact.survivors.is_empty()
            {
                return Err(CampaignError::ArtifactSchema);
            }
        }
        CampaignKind::Mutation => {
            if artifact.accepted_mutants == 0
                || artifact.killed_mutants != artifact.accepted_mutants
                || artifact.score_basis_points != 10_000
            {
                return Err(CampaignError::MutationScore);
            }
        }
    }
    Ok(())
}

fn validate_dates(
    generated: &str,
    expires: &str,
    as_of: &str,
    maximum_age_days: u64,
) -> Result<(), CampaignError> {
    let generated = civil_day(generated).ok_or(CampaignError::ArtifactSchema)?;
    let expires = civil_day(expires).ok_or(CampaignError::ArtifactSchema)?;
    let as_of = civil_day(as_of).ok_or(CampaignError::ArtifactSchema)?;
    // `parse_policy` has already closed this value to 1..=90.
    let maximum = i64::try_from(maximum_age_days).unwrap_or(i64::MAX);
    if expires < generated || as_of < generated || as_of > expires || expires - generated > maximum
    {
        return Err(CampaignError::Expired);
    }
    Ok(())
}

fn civil_day(value: &str) -> Option<i64> {
    let bytes = value.as_bytes();
    if bytes.len() != 10 || bytes[4] != b'-' || bytes[7] != b'-' {
        return None;
    }
    let year = decimal(&bytes[0..4])?;
    let month = decimal(&bytes[5..7])?;
    let day = decimal(&bytes[8..10])?;
    if year < 1970 || !(1..=12).contains(&month) {
        return None;
    }
    let leap = year % 4 == 0 && (year % 100 != 0 || year % 400 == 0);
    let maximum = match month {
        2 if leap => 29,
        2 => 28,
        4 | 6 | 9 | 11 => 30,
        _ => 31,
    };
    if !(1..=maximum).contains(&day) {
        return None;
    }
    let adjusted_year = year - i64::from(month <= 2);
    let era = adjusted_year.div_euclid(400);
    let year_of_era = adjusted_year - era * 400;
    let adjusted_month = month + if month > 2 { -3 } else { 9 };
    let day_of_year = (153 * adjusted_month + 2) / 5 + day - 1;
    let day_of_era = year_of_era * 365 + year_of_era / 4 - year_of_era / 100 + day_of_year;
    Some(era * 146_097 + day_of_era - 719_468)
}

fn decimal(bytes: &[u8]) -> Option<i64> {
    bytes.iter().try_fold(0_i64, |value, byte| {
        byte.is_ascii_digit()
            .then_some(value * 10 + i64::from(byte - b'0'))
    })
}

fn has_duplicates(values: &[String]) -> bool {
    values.iter().collect::<BTreeSet<_>>().len() != values.len()
}

fn digest_declared_paths(
    root: &Path,
    declared: &[String],
) -> Result<SemanticDigest, CampaignError> {
    digest_declared_paths_with_limits(root, declared, MAX_BOUND_FILES, MAX_BOUND_BYTES, None)
}

fn digest_declared_paths_with_limits(
    root: &Path,
    declared: &[String],
    max_files: usize,
    max_bytes: u64,
    fault: Option<&str>,
) -> Result<SemanticDigest, CampaignError> {
    let mut files = Vec::new();
    let mut total_bytes = 0_u64;
    for relative in declared {
        if RepositoryPath::new(relative).is_err() {
            return Err(CampaignError::BoundInput);
        }
        collect_files(
            root,
            &root.join(relative),
            Path::new(relative),
            &mut files,
            &mut total_bytes,
            max_files,
            max_bytes,
            fault,
        )?;
    }
    files.sort_unstable_by(|left, right| left.0.cmp(&right.0));
    if files.is_empty() || files.windows(2).any(|pair| pair[0].0 == pair[1].0) {
        return Err(CampaignError::BoundInput);
    }
    let mut entries = Vec::with_capacity(files.len());
    for (path, bytes) in files {
        let content = match (
            fault == Some("file-digest"),
            semantic_digest(
                "campaign-input-file",
                vec![(1, CanonicalValue::Bytes(bytes))],
            ),
        ) {
            (true, _) | (_, Err(_)) => return Err(CampaignError::BoundInput),
            (false, Ok(digest)) => digest,
        };
        entries.push(CanonicalValue::Record(vec![
            (1, CanonicalValue::Text(path)),
            (2, CanonicalValue::Digest(content)),
        ]));
    }
    match (
        fault == Some("set-digest"),
        semantic_digest(
            "campaign-input-set",
            vec![(1, CanonicalValue::Set(entries))],
        ),
    ) {
        (true, _) | (_, Err(_)) => Err(CampaignError::BoundInput),
        (false, Ok(digest)) => Ok(digest),
    }
}

#[allow(clippy::too_many_arguments)]
fn collect_files(
    root: &Path,
    path: &Path,
    relative: &Path,
    output: &mut Vec<(String, Vec<u8>)>,
    total_bytes: &mut u64,
    max_files: usize,
    max_bytes: u64,
    fault: Option<&str>,
) -> Result<(), CampaignError> {
    let metadata = fs::symlink_metadata(path).map_err(|_| CampaignError::BoundInput)?;
    if metadata.file_type().is_symlink() {
        return Err(CampaignError::BoundInput);
    }
    if metadata.is_file() {
        if output.len() >= max_files {
            return Err(CampaignError::BoundInput);
        }
        *total_bytes = total_bytes
            .checked_add(metadata.len())
            .filter(|total| *total <= max_bytes)
            .ok_or(CampaignError::BoundInput)?;
        let text = path_text(relative)?;
        let read_path = if fault == Some("read-file") {
            root.join("missing-read-control")
        } else {
            path.to_path_buf()
        };
        let bytes = fs::read(read_path).map_err(|_| CampaignError::BoundInput)?;
        output.push((text, bytes));
        return Ok(());
    }
    if !metadata.is_dir() {
        return Err(CampaignError::BoundInput);
    }
    let directory_path = if fault == Some("read-directory") {
        root.join("missing-directory-control")
    } else {
        path.to_path_buf()
    };
    let entries = fs::read_dir(directory_path).map_err(|_| CampaignError::BoundInput)?;
    let collected = if fault == Some("read-entry") {
        Err(io::Error::other("campaign entry coverage control"))
    } else {
        entries
            .map(|entry| {
                entry.map(|value| {
                    let child_relative = relative.join(value.file_name());
                    (value.path(), child_relative)
                })
            })
            .collect::<Result<Vec<_>, _>>()
    };
    let mut children = collected.map_err(|_| CampaignError::BoundInput)?;
    children.sort_unstable();
    for (child, child_relative) in children {
        collect_files(
            root,
            &child,
            &child_relative,
            output,
            total_bytes,
            max_files,
            max_bytes,
            fault,
        )?;
    }
    Ok(())
}

fn path_text(path: &Path) -> Result<String, CampaignError> {
    let mut components = Vec::new();
    for component in path.components() {
        components.push(
            component
                .as_os_str()
                .to_str()
                .ok_or(CampaignError::BoundInput)?,
        );
    }
    let text = components.join("/");
    RepositoryPath::new(&text).map_err(|_| CampaignError::BoundInput)?;
    Ok(text)
}

/// Runs campaign input collection with an explicit deterministic test bound.
///
/// # Errors
///
/// Returns [`CampaignError::BoundInput`] when the supplied file or byte bound
/// is exceeded.
#[cfg(feature = "coverage-controls")]
#[doc(hidden)]
pub fn campaign_bound_control(
    root: &Path,
    declared: &[String],
    max_files: usize,
    max_bytes: u64,
) -> Result<(), CampaignError> {
    digest_declared_paths_with_limits(root, declared, max_files, max_bytes, None).map(|_| ())
}

/// Runs one owned campaign input I/O or digest failure stage.
///
/// # Errors
///
/// Returns [`CampaignError::BoundInput`] for every recognized injected fault.
#[cfg(feature = "coverage-controls")]
#[doc(hidden)]
pub fn campaign_input_fault_control(
    root: &Path,
    declared: &[String],
    fault: &str,
) -> Result<(), CampaignError> {
    digest_declared_paths_with_limits(
        root,
        declared,
        MAX_BOUND_FILES,
        MAX_BOUND_BYTES,
        Some(fault),
    )
    .map(|_| ())
}

/// Runs the repository-component UTF-8 boundary used by campaign binding.
///
/// # Errors
///
/// Returns [`CampaignError::BoundInput`] for a nonportable component.
#[cfg(feature = "coverage-controls")]
#[doc(hidden)]
pub fn repository_path_text_control(path: &Path) -> Result<String, CampaignError> {
    path_text(path)
}

fn validate_fuzz_layout(root: &Path, policy: &CampaignPolicy) -> Result<(), CampaignError> {
    let manifest = fs::read_to_string(root.join("fuzz/Cargo.toml"))
        .map_err(|_| CampaignError::PolicySchema)?;
    for unit in &policy.required_units {
        if !manifest.contains(&format!("name = \"{unit}\""))
            || !root.join(format!("fuzz/fuzz_targets/{unit}.rs")).is_file()
        {
            return Err(CampaignError::PolicySchema);
        }
        let corpus = root.join(format!("fuzz/corpus/{unit}"));
        let count = fs::read_dir(corpus)
            .map_err(|_| CampaignError::PolicySchema)?
            .filter_map(Result::ok)
            .filter(|entry| entry.path().is_file())
            .count();
        if count < 2 {
            return Err(CampaignError::PolicySchema);
        }
    }
    Ok(())
}

fn validate_mutation_layout(root: &Path) -> Result<(), CampaignError> {
    let configuration = fs::read_to_string(root.join(".cargo/mutants.toml"))
        .map_err(|_| CampaignError::PolicySchema)?;
    for required in [
        "crates/buoy-core/src/value.rs",
        "crates/buoy-core/src/algorithms.rs",
        "crates/buoy-core/src/report.rs",
        "src/repository.rs",
        "src/oracle.rs",
    ] {
        if !configuration.contains(required) {
            return Err(CampaignError::PolicySchema);
        }
    }
    if configuration
        .lines()
        .any(|line| line.trim_start().starts_with("exclude_re") && line.trim() != "exclude_re = []")
        || configuration.contains("exclude_globs = [\n  \"crates/buoy-core/src/value.rs")
    {
        return Err(CampaignError::PolicySchema);
    }
    Ok(())
}

/// Validates the exact named handcrafted mutation catalog and its source links.
///
/// # Errors
///
/// Rejects missing high-consequence classes, duplicate IDs/categories, excluded
/// required rows, missing source symbols, or malformed commands/dispositions.
pub fn validate_named_mutant_catalog(root: &Path) -> Result<(), CampaignError> {
    let text =
        fs::read_to_string(root.join(NAMED_MUTANTS)).map_err(|_| CampaignError::NamedMutants)?;
    let mut lines = text.lines();
    if lines.next()
        != Some(
            "schema\tid\tcategory\tsource\tproduction_symbol\ttest_command\texcluded\tdisposition",
        )
    {
        return Err(CampaignError::NamedMutants);
    }
    let required = [
        "artifact-freshness",
        "strict-decoding",
        "complete-impacts",
        "policy-derivation",
        "process-cleanup",
        "output-enforcement",
        "budget-enforcement",
    ];
    let retained = [
        "lifecycle",
        "surface-completeness",
        "freshness",
        "canonical-boundary",
        "policy-monotonicity",
        "impact-closure",
        "no-laundering",
        "aggregation",
    ];
    let mut ids = BTreeSet::new();
    let mut categories = BTreeSet::new();
    for line in lines {
        let fields = line.split('\t').collect::<Vec<_>>();
        if fields.len() != 8
            || fields[0] != "buoy-named-mutant/v1"
            || !ids.insert(fields[1])
            || !categories.insert(fields[2])
            || fields[6] != "false"
            || fields[7] != "blocking"
            || !fields[5].starts_with("cargo test ")
        {
            return Err(CampaignError::NamedMutants);
        }
        let source =
            fs::read_to_string(root.join(fields[3])).map_err(|_| CampaignError::NamedMutants)?;
        if fields[4].is_empty() || !source.contains(fields[4]) {
            return Err(CampaignError::NamedMutants);
        }
    }
    if required
        .iter()
        .chain(retained.iter())
        .any(|category| !categories.contains(category))
    {
        return Err(CampaignError::NamedMutants);
    }
    Ok(())
}

/// Produces the exact host-independent portable canonical corpus summary.
///
/// # Errors
///
/// Returns a corpus error if a supposedly valid fixed input stops encoding,
/// parsing, or round-tripping.
pub fn portable_corpus() -> Result<String, CampaignError> {
    portable_corpus_inner(None)
}

/// Runs one deterministic portable-corpus failure control.
///
/// This API exists only for assurance coverage and never changes the production
/// corpus path.
///
/// # Errors
///
/// Returns the exact closed corpus category selected by `control`.
#[cfg(feature = "coverage-controls")]
#[doc(hidden)]
pub fn portable_corpus_control(control: &str) -> Result<String, CampaignError> {
    portable_corpus_inner(Some(control))
}

#[allow(clippy::too_many_lines)]
fn portable_corpus_inner(control: Option<&str>) -> Result<String, CampaignError> {
    let mut rows = Vec::new();
    let value = if control == Some("canonical-encode") {
        CanonicalValue::Record(vec![
            (1, CanonicalValue::U64(1)),
            (1, CanonicalValue::U64(2)),
        ])
    } else {
        CanonicalValue::Record(vec![
            (1, CanonicalValue::Text(String::from("portable"))),
            (2, CanonicalValue::U64(7)),
        ])
    };
    let value_bytes = value.encode().map_err(|_| CampaignError::CorpusCanonical)?;
    let decoded_value = if control == Some("canonical-decode") {
        Ok(CanonicalValue::U64(0))
    } else {
        CanonicalValue::decode(
            &value_bytes,
            DecodeBudget {
                max_bytes: 4_096,
                max_nesting: 16,
                max_values: 64,
            },
        )
    };
    if decoded_value != Ok(value) {
        return Err(CampaignError::CorpusCanonical);
    }
    rows.push(corpus_row(
        "canonical-values",
        &value_bytes,
        control == Some("canonical-row"),
    )?);

    let path_text = if control == Some("path-construct") {
        "../escape"
    } else {
        "spec/Straße.md"
    };
    let path = RepositoryPath::new(path_text).map_err(|_| CampaignError::CorpusPath)?;
    let glob_text = if control == Some("glob-parse") {
        "a/[ab].md"
    } else {
        "**/*.md"
    };
    let glob = PortableGlob::parse(glob_text).map_err(|_| CampaignError::CorpusPath)?;
    let path_observation = format!("{}\t{}", path.portable_collision_key(), glob.matches(&path));
    rows.push(corpus_row(
        "paths-globs",
        path_observation.as_bytes(),
        control == Some("path-row"),
    )?);

    let markdown_path = if control == Some("markdown-path") {
        "../markdown"
    } else {
        "spec/portable.md"
    };
    let markdown = extract_markdown(
        &RepositoryPath::new(markdown_path).map_err(|_| CampaignError::Corpus)?,
        b"portable prose\n",
        AuthorityExpectation::Optional,
    );
    let rust_path = if control == Some("rust-path") {
        "../rust"
    } else {
        "src/portable.rs"
    };
    let rust = extract_rust(
        &RepositoryPath::new(rust_path).map_err(|_| CampaignError::Corpus)?,
        b"fn portable() {}\n",
    );
    let adapters = format!(
        "{}:{}:{}:{}",
        markdown.semantic.len(),
        markdown.nonnormative_nodes,
        rust.semantic.len(),
        rust.nonnormative_items
    );
    rows.push(corpus_row(
        "adapters",
        adapters.as_bytes(),
        control == Some("adapter-row"),
    )?);

    let config = if control == Some("configuration-parse") {
        b"invalid".as_slice()
    } else {
        b"schema = \"spec-provenance-config/v2\"\ndigest_algorithms = [\"xxh3-128\"]\n[[selectors]]\nroot = \"spec\"\nadapter = \"markdown/v1\"\ninclude = [\"**/*.md\"]\nexclude = []\nauthority_expectation = \"optional\"\n".as_slice()
    };
    let config = RepositoryConfig::parse(config).map_err(|_| CampaignError::CorpusConfiguration)?;
    let config_digest = match (
        control == Some("configuration-digest"),
        config.semantic_digest(),
    ) {
        (true, _) | (_, Err(_)) => return Err(CampaignError::CorpusConfiguration),
        (false, Ok(digest)) => digest,
    };
    let manifest = if control == Some("configuration-mismatch") {
        b"invalid".as_slice()
    } else {
        b"schema = \"spec-provenance-change-manifest/v1\"\nid = \"CHANGE-PORTABLE-001\"\nbase_graph = \"xxh3-128:00000000000000000000000000000000\"\nhead_graph = \"xxh3-128:00000000000000000000000000000000\"\ndispositions = []\n".as_slice()
    };
    if !crate::cli::validate_change_manifest_bytes(manifest) {
        return Err(CampaignError::CorpusConfiguration);
    }
    rows.push(corpus_row(
        "config-manifests",
        config_digest.canonical_text().as_bytes(),
        control == Some("configuration-row"),
    )?);

    let mut classes = aggregate_assurance(&AggregationInput {
        linked: true,
        required_fresh: true,
        required_selected: true,
        verdicts: vec![Verdict::Pass],
        exhaustive: false,
        formal: false,
        structural: true,
        reviewed: false,
        debt: true,
        waiver: true,
        reaffirmation: true,
    });
    if control == Some("governance-mismatch") {
        classes.clear();
    }
    if classes
        != vec![
            AssuranceClass::Linked,
            AssuranceClass::Tested,
            AssuranceClass::StructurallyEnforced,
        ]
    {
        return Err(CampaignError::CorpusGovernance);
    }
    rows.push(corpus_row(
        "graphs-governance",
        b"linked,tested,structural",
        control == Some("governance-row"),
    )?);

    let report = match (
        control == Some("report-new"),
        CanonicalStaticReport::new(
            SemanticDigest::xxh3_128([1; 16]),
            Vec::new(),
            Vec::new(),
            Vec::new(),
            Vec::new(),
            Vec::new(),
            Vec::new(),
            Vec::new(),
            Vec::new(),
            Vec::new(),
        ),
    ) {
        (true, _) | (_, Err(_)) => return Err(CampaignError::CorpusReport),
        (false, Ok(report)) => report,
    };
    let report_bytes = if control == Some("report-decode") {
        b"invalid".as_slice()
    } else {
        report.bytes()
    };
    let decoded = CanonicalStaticReport::decode(
        report_bytes,
        DecodeBudget {
            max_bytes: 4_096,
            max_nesting: 16,
            max_values: 64,
        },
    )
    .map_err(|_| CampaignError::CorpusReport)?;
    let expected_report = if control == Some("report-mismatch") {
        b"different".as_slice()
    } else {
        report.bytes()
    };
    if decoded.bytes() != expected_report {
        return Err(CampaignError::CorpusReport);
    }
    rows.push(corpus_row(
        "reports",
        report.bytes(),
        control == Some("report-row"),
    )?);

    let mut output = String::from("schema\tscope\tdigest\n");
    for row in rows {
        output.push_str(&row);
    }
    Ok(output)
}

fn corpus_row(scope: &str, bytes: &[u8], fail: bool) -> Result<String, CampaignError> {
    let digest = match (
        fail,
        semantic_digest(
            "adversarial-portable-corpus-row",
            vec![
                (1, CanonicalValue::Text(scope.to_string())),
                (2, CanonicalValue::Bytes(bytes.to_vec())),
            ],
        ),
    ) {
        (true, _) | (_, Err(_)) => return Err(CampaignError::Corpus),
        (false, Ok(digest)) => digest,
    };
    Ok(format!(
        "buoy-adversarial-portable-corpus/v1\t{scope}\t{}\n",
        digest.canonical_text()
    ))
}

/// Emits stable platform-effect support or blocker categories without changing
/// the portable canonical corpus.
///
/// # Errors
///
/// Rejects an undeclared platform label.
pub fn platform_effect_diagnostics(platform: &str) -> Result<String, CampaignError> {
    let rows = match platform {
        "linux" | "macos" => [
            ("snapshot", "descriptor-relative-supported"),
            ("process", "process-group-supported"),
        ],
        "windows" => [
            ("snapshot", "PLATFORM-WINDOWS-SNAPSHOT-001"),
            ("process", "PLATFORM-WINDOWS-JOB-001"),
        ],
        _ => return Err(CampaignError::Platform),
    };
    let mut output = String::from("schema\tplatform\teffect\tdiagnostic\n");
    for (effect, diagnostic) in rows {
        let _ = writeln!(
            output,
            "buoy-platform-effect/v1\t{platform}\t{effect}\t{diagnostic}"
        );
    }
    Ok(output)
}

#[cfg(test)]
mod date_coverage_tests {
    use super::{CampaignError, validate_dates};

    #[test]
    fn campaign_dates_reject_reverse_and_pre_generation_ranges() {
        assert_eq!(
            validate_dates("2026-08-26", "2026-08-25", "2026-08-26", 30),
            Err(CampaignError::Expired)
        );
        assert_eq!(
            validate_dates("2026-08-26", "2026-09-25", "2026-08-25", 30),
            Err(CampaignError::Expired)
        );
        assert_eq!(
            validate_dates("2026-08-01", "2026-09-25", "2026-08-02", 30),
            Err(CampaignError::Expired)
        );
        assert_eq!(
            validate_dates("2026-08-26", "2026-09-25", "2026-09-26", 30),
            Err(CampaignError::Expired)
        );
    }
}
