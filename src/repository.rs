//! Effectful configuration loading and explicit repository snapshot construction.

use crate::adapters::{AuthorityExpectation, OpenSpecConfiguration};
use crate::glob::PortableGlob;
use buoy_core::{
    ActiveAlgorithm, ActiveAlgorithmError, ArtifactEntry, ArtifactIndex, CanonicalValue,
    DigestAlgorithm, DigestAssuranceClass, EncodingError, LogicalMount, LogicalMountError,
    LogicalMountId, RawArtifactMediaType, RejectedEntryObservation, RepositoryPath, SemanticDigest,
    SourceBudgets, SourceDepthBudget, SourceEdgeBudget, SourceNodeBudget, SourceVisitedBudget,
    canonical_logical_mounts, canonical_paths, mount_relative_path, project_logical_path,
    semantic_digest_with,
};
#[cfg(unix)]
use rustix::fd::OwnedFd;
#[cfg(unix)]
use rustix::fs::{Dir, FileType, Mode, OFlags, fstat, open, openat};
use serde::Deserialize;
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::io::Read as _;
use std::path::{Path, PathBuf};

const LEGACY_CONFIG_SCHEMA: &str = "spec-provenance-config/v1";
const CONFIG_SCHEMA: &str = "spec-provenance-config/v2";
const MOUNT_CONFIG_SCHEMA: &str = "buoy/repository-config/v3";

/// Closed repository configuration schema.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RepositoryConfig {
    schema: String,
    active_algorithm: ActiveAlgorithm,
    mounts: Vec<LogicalMount>,
    selectors: Vec<Selector>,
    oracles: Vec<OracleCommand>,
    adapter_configurations: BTreeMap<String, String>,
    budgets: SnapshotBudgets,
    source_budgets: SourceBudgets,
    oracle_concurrency: u64,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RepositoryConfigInput {
    schema: String,
    #[serde(default)]
    digest_algorithms: Vec<String>,
    #[serde(default)]
    mounts: Vec<MountInput>,
    selectors: Vec<SelectorInput>,
    #[serde(default)]
    oracles: Vec<OracleCommand>,
    #[serde(default)]
    adapter_configurations: BTreeMap<String, String>,
    #[serde(default)]
    budgets: SnapshotBudgets,
    #[serde(default = "default_oracle_concurrency")]
    oracle_concurrency: u64,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct MountInput {
    id: String,
    prefix: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct SelectorInput {
    #[serde(default)]
    mount: Option<String>,
    root: String,
    adapter: String,
    include: Vec<String>,
    #[serde(default)]
    exclude: Vec<String>,
    authority_expectation: String,
}

impl RepositoryConfig {
    /// Parses strict repository-owned TOML configuration.
    ///
    /// # Errors
    ///
    /// Returns a stable configuration error for invalid UTF-8, TOML, schema,
    /// selectors, or oracle declarations.
    pub fn parse(bytes: &[u8]) -> Result<Self, RepositoryError> {
        Self::parse_internal(bytes, false)
    }

    #[allow(clippy::expect_used, clippy::too_many_lines)]
    fn parse_internal(bytes: &[u8], force_unavailable: bool) -> Result<Self, RepositoryError> {
        if bytes.len() > CONFIGURATION_BYTES_MAXIMUM {
            return Err(RepositoryError::ConfigBytesBudget);
        }
        let source = core::str::from_utf8(bytes).map_err(|_| RepositoryError::ConfigUtf8)?;
        let input = toml::from_str::<RepositoryConfigInput>(source)
            .map_err(|_| RepositoryError::ConfigSyntax)?;
        let [algorithm_name] = input.digest_algorithms.as_slice() else {
            return Err(if input.digest_algorithms.is_empty() {
                RepositoryError::DigestActiveMissing
            } else {
                RepositoryError::DigestActiveMultiple
            });
        };
        let algorithm = DigestAlgorithm::from_canonical_name(algorithm_name)
            .ok_or(RepositoryError::DigestAlgorithmUnknown)?;
        let active_result = if force_unavailable {
            Err(ActiveAlgorithmError::Unavailable(algorithm))
        } else {
            ActiveAlgorithm::new(algorithm)
        };
        let active_algorithm = repository_active_result(active_result)?;
        if input.schema == LEGACY_CONFIG_SCHEMA
            || !matches!(input.schema.as_str(), CONFIG_SCHEMA | MOUNT_CONFIG_SCHEMA)
        {
            return Err(RepositoryError::ConfigSchema);
        }
        let mounted = input.schema == MOUNT_CONFIG_SCHEMA;
        let mounts = parse_mount_declarations(mounted, input.mounts)?;
        let selectors = parse_selectors(mounted, input.selectors, &mounts)?;
        let config = Self {
            schema: input.schema,
            active_algorithm,
            mounts,
            selectors,
            oracles: input.oracles,
            adapter_configurations: input.adapter_configurations,
            budgets: input.budgets,
            source_budgets: input.budgets.source_budgets()?,
            oracle_concurrency: input.oracle_concurrency,
        };
        config.budgets.validate()?;
        if !(1..=64).contains(&config.oracle_concurrency) {
            return Err(RepositoryError::OracleConcurrencyBudget);
        }
        for (adapter, bytes) in &config.adapter_configurations {
            if adapter == "artifact/v1" {
                return Err(RepositoryError::ArtifactConfiguration);
            }
            if adapter != "openspec/v1" || OpenSpecConfiguration::parse(bytes.as_bytes()).is_err() {
                return Err(RepositoryError::ConfigSchema);
            }
        }
        if config
            .selectors
            .iter()
            .any(|selector| selector.adapter == "openspec/v1")
            && !config.adapter_configurations.contains_key("openspec/v1")
        {
            return Err(RepositoryError::ConfigSchema);
        }
        if config
            .selectors
            .iter()
            .any(|selector| selector.adapter == "binding/v1")
            && config.active_algorithm.algorithm().assurance_class()
                != DigestAssuranceClass::CollisionResistant
        {
            // A compact locator names one algorithm-qualified canonical digest
            // with no separate agility field, so compact binding selection
            // requires the collision-resistant class. The
            // selected member itself computes every binding record identity, so
            // no build-default member takes part in that decision.
            return Err(RepositoryError::BindingDigestClass);
        }
        let mut oracle_ids = BTreeSet::new();
        for oracle in &config.oracles {
            if !oracle_ids.insert(&oracle.id) {
                return Err(RepositoryError::ConfigSchema);
            }
            if oracle.timeout_ms == 0 || oracle.timeout_ms > 3_600_000 {
                return Err(RepositoryError::OracleTimeoutBudget);
            }
            if oracle.stdout_limit == 0
                || oracle.stdout_limit > 67_108_864
                || oracle.stderr_limit == 0
                || oracle.stderr_limit > 67_108_864
                || oracle.combined_output_limit == 0
                || oracle.combined_output_limit > 134_217_728
                || oracle.raw_log_retention > 8_388_608
            {
                return Err(RepositoryError::OracleOutputBudget);
            }
            if oracle.result_protocol != "spec-provenance-result/v1" {
                return Err(RepositoryError::OracleProtocol);
            }
            if mounted {
                let working_mount = LogicalMountId::new(&oracle.working_root)
                    .map_err(|_| RepositoryError::OracleMount)?;
                if !config
                    .mounts
                    .iter()
                    .any(|mount| mount.id() == &working_mount)
                {
                    return Err(RepositoryError::OracleMount);
                }
            } else if oracle.working_root != "repository-root" {
                return Err(RepositoryError::OracleProtocol);
            }
            if oracle.environment.keys().any(|name| {
                name.is_empty()
                    || name.contains('=')
                    || !name
                        .bytes()
                        .all(|byte| byte.is_ascii_uppercase() || byte == b'_')
            }) {
                return Err(RepositoryError::ConfigSchema);
            }
        }
        config
            .semantic_digest()
            .expect("validated repository configuration fields always encode");
        Ok(config)
    }

    /// The parsed active digest member.
    #[must_use]
    pub const fn active_algorithm(&self) -> ActiveAlgorithm {
        self.active_algorithm
    }

    /// Reports whether this configuration uses explicit logical mounts.
    #[must_use]
    pub fn is_mounted(&self) -> bool {
        self.schema == MOUNT_CONFIG_SCHEMA
    }

    /// Canonical logical mount declarations.
    #[must_use]
    pub fn mounts(&self) -> &[LogicalMount] {
        &self.mounts
    }

    /// Returns one declared mount by its parsed ID.
    #[must_use]
    pub fn mount(&self, id: &LogicalMountId) -> Option<&LogicalMount> {
        self.mounts.iter().find(|mount| mount.id() == id)
    }

    /// Resolves one projected logical path to its unique declaration.
    #[must_use]
    pub fn mount_for_logical_path(&self, path: &RepositoryPath) -> Option<&LogicalMount> {
        self.mounts
            .iter()
            .find(|mount| mount_relative_path(mount, path).is_some())
    }

    /// Explicit configured selectors.
    #[must_use]
    pub fn selectors(&self) -> &[Selector] {
        &self.selectors
    }

    /// Returns exact adapter configuration bytes for one adapter ID.
    #[must_use]
    pub fn adapter_configuration(&self, adapter_id: &str) -> Option<&[u8]> {
        self.adapter_configurations
            .get(adapter_id)
            .map(String::as_bytes)
    }

    /// Explicit effectful oracle commands.
    #[must_use]
    pub fn oracles(&self) -> &[OracleCommand] {
        &self.oracles
    }

    /// Explicit snapshot resource limits.
    #[must_use]
    pub const fn budgets(&self) -> SnapshotBudgets {
        self.budgets
    }

    /// Parsed positive source-graph limits for repository-level aggregation.
    #[must_use]
    pub const fn source_budgets(&self) -> SourceBudgets {
        self.source_budgets
    }

    /// Maximum concurrently supervised oracle processes.
    #[must_use]
    pub const fn oracle_concurrency(&self) -> u64 {
        self.oracle_concurrency
    }

    /// Canonical identity of all acceptance-affecting repository configuration.
    ///
    /// # Errors
    ///
    /// Returns an encoding error only if a configured set is noncanonical.
    pub fn semantic_digest(&self) -> Result<SemanticDigest, EncodingError> {
        let mut fields = vec![
            (1, CanonicalValue::Text(self.schema.clone())),
            (
                2,
                CanonicalValue::Text(
                    self.active_algorithm
                        .algorithm()
                        .canonical_name()
                        .to_owned(),
                ),
            ),
            (
                3,
                CanonicalValue::Set(
                    self.selectors
                        .iter()
                        .map(Selector::canonical_value)
                        .collect(),
                ),
            ),
            (
                4,
                CanonicalValue::Set(
                    self.oracles
                        .iter()
                        .map(OracleCommand::canonical_value)
                        .collect(),
                ),
            ),
            (5, self.budgets.canonical_value()),
            (6, CanonicalValue::U64(self.oracle_concurrency)),
            (
                7,
                CanonicalValue::Set(
                    self.adapter_configurations
                        .iter()
                        .map(|(adapter, bytes)| {
                            CanonicalValue::Record(vec![
                                (1, CanonicalValue::Text(adapter.clone())),
                                (2, CanonicalValue::Bytes(bytes.as_bytes().to_vec())),
                            ])
                        })
                        .collect(),
                ),
            ),
        ];
        if self.is_mounted() {
            fields.push((
                8,
                CanonicalValue::Set(
                    self.mounts
                        .iter()
                        .map(LogicalMount::canonical_value)
                        .collect(),
                ),
            ));
        }
        semantic_digest_with(
            self.active_algorithm,
            if self.is_mounted() {
                "buoy/repository-config/v3"
            } else {
                "repository-execution-config"
            },
            fields,
        )
    }
}

fn parse_mount_declarations(
    mounted: bool,
    inputs: Vec<MountInput>,
) -> Result<Vec<LogicalMount>, RepositoryError> {
    if !mounted {
        return if inputs.is_empty() {
            Ok(Vec::new())
        } else {
            Err(RepositoryError::ConfigSchema)
        };
    }
    if inputs.is_empty() {
        return Err(RepositoryError::MountMissing);
    }
    let mounts = inputs
        .into_iter()
        .map(|input| {
            let id = LogicalMountId::new(&input.id).map_err(|_| RepositoryError::MountId)?;
            let prefix =
                RepositoryPath::new(&input.prefix).map_err(|_| RepositoryError::MountPrefix)?;
            Ok(LogicalMount::new(id, prefix))
        })
        .collect::<Result<Vec<_>, _>>()?;
    canonical_logical_mounts(mounts).map_err(project_mount_error)
}

const fn project_mount_error(error: LogicalMountError) -> RepositoryError {
    match error {
        LogicalMountError::DuplicateId => RepositoryError::MountIdDuplicate,
        LogicalMountError::PrefixOverlap => RepositoryError::MountPrefixOverlap,
        LogicalMountError::Id => RepositoryError::MountId,
        LogicalMountError::Projection => RepositoryError::MountPrefix,
    }
}

fn parse_selectors(
    mounted: bool,
    inputs: Vec<SelectorInput>,
    mounts: &[LogicalMount],
) -> Result<Vec<Selector>, RepositoryError> {
    if inputs.is_empty() {
        return Err(RepositoryError::ConfigSchema);
    }
    let declared = mounts
        .iter()
        .map(|mount| mount.id().clone())
        .collect::<BTreeSet<_>>();
    let mut selectors = inputs
        .into_iter()
        .map(|input| {
            let mount = match (mounted, input.mount) {
                (true, Some(value)) => {
                    let id =
                        LogicalMountId::new(&value).map_err(|_| RepositoryError::SelectorMount)?;
                    if !declared.contains(&id) {
                        return Err(RepositoryError::SelectorMount);
                    }
                    Some(id)
                }
                (true, None) => return Err(RepositoryError::SelectorMount),
                (false, None) => None,
                (false, Some(_)) => return Err(RepositoryError::ConfigSchema),
            };
            let root_identity = input.root;
            let root =
                RepositoryPath::new(&root_identity).map_err(|_| RepositoryError::PathPolicy)?;
            let include = parse_globs(input.include)?;
            if include.is_empty() {
                return Err(RepositoryError::ConfigSchema);
            }
            let exclude = parse_globs(input.exclude)?;
            let authority_expectation = parse_authority_expectation(&input.authority_expectation)?;
            if input.adapter == "artifact/v1"
                && authority_expectation != AuthorityExpectation::Forbidden
            {
                return Err(RepositoryError::ArtifactAuthority);
            }
            Ok(Selector {
                mount,
                root,
                root_identity,
                adapter: input.adapter,
                include,
                exclude,
                authority_expectation,
            })
        })
        .collect::<Result<Vec<_>, _>>()?;
    let mut encoded = selectors
        .drain(..)
        .map(|selector| {
            selector
                .canonical_value()
                .encode()
                .map(|bytes| (bytes, selector))
                .map_err(|_| RepositoryError::ConfigSchema)
        })
        .collect::<Result<Vec<_>, _>>()?;
    encoded.sort_unstable_by(|left, right| left.0.cmp(&right.0));
    if encoded.windows(2).any(|pair| pair[0].0 == pair[1].0) {
        return Err(RepositoryError::ConfigSchema);
    }
    Ok(encoded.into_iter().map(|(_, selector)| selector).collect())
}

fn parse_globs(values: Vec<String>) -> Result<Vec<PortableGlob>, RepositoryError> {
    let mut parsed = values
        .into_iter()
        .map(|value| PortableGlob::parse(&value).map_err(|_| RepositoryError::GlobSyntax))
        .collect::<Result<Vec<_>, _>>()?;
    parsed.sort_unstable_by(|left, right| left.as_str().cmp(right.as_str()));
    if parsed
        .windows(2)
        .any(|pair| pair[0].as_str() == pair[1].as_str())
    {
        return Err(RepositoryError::ConfigSchema);
    }
    Ok(parsed)
}

fn parse_authority_expectation(value: &str) -> Result<AuthorityExpectation, RepositoryError> {
    match value {
        "forbidden" => Ok(AuthorityExpectation::Forbidden),
        "optional" => Ok(AuthorityExpectation::Optional),
        "required" => Ok(AuthorityExpectation::Required),
        _ => Err(RepositoryError::ConfigSchema),
    }
}

const CONFIGURATION_BYTES_MAXIMUM: usize = 1_048_576;
#[cfg(any(windows, feature = "coverage-controls"))]
const WINDOWS_FILE_ATTRIBUTE_REPARSE_POINT: u32 = 0x0000_0400;

const fn default_oracle_concurrency() -> u64 {
    8
}

/// Explicit snapshot resource limits in counts and bytes.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct SnapshotBudgets {
    /// Maximum path-component depth before descent.
    pub path_depth: u64,
    /// Maximum observed directory entries.
    pub directory_entries: u64,
    /// Maximum selected regular files.
    pub selected_files: u64,
    /// Maximum bytes in one selected file.
    pub file_bytes: u64,
    /// Maximum aggregate selected bytes.
    pub total_bytes: u64,
    /// Maximum materialized semantic graph objects.
    #[serde(default = "default_graph_objects")]
    pub graph_objects: u64,
    /// Maximum direct semantic graph references.
    #[serde(default = "default_graph_references")]
    pub graph_references: u64,
    /// Maximum retained stable diagnostics.
    #[serde(default = "default_diagnostics")]
    pub diagnostics: u64,
    /// Maximum aggregate source-graph nodes.
    #[serde(default = "default_source_nodes")]
    pub source_nodes: u64,
    /// Maximum aggregate source-graph edges.
    #[serde(default = "default_source_edges")]
    pub source_edges: u64,
    /// Maximum traversed source-graph depth in edges.
    #[serde(default = "default_source_depth")]
    pub source_depth: u64,
    /// Maximum visited nodes in one source-graph traversal.
    #[serde(default = "default_source_visited")]
    pub source_visited: u64,
}

impl Default for SnapshotBudgets {
    fn default() -> Self {
        Self {
            path_depth: 64,
            directory_entries: 100_000,
            selected_files: 10_000,
            file_bytes: 16_777_216,
            total_bytes: 536_870_912,
            graph_objects: default_graph_objects(),
            graph_references: default_graph_references(),
            diagnostics: default_diagnostics(),
            source_nodes: default_source_nodes(),
            source_edges: default_source_edges(),
            source_depth: default_source_depth(),
            source_visited: default_source_visited(),
        }
    }
}

impl SnapshotBudgets {
    fn canonical_value(self) -> CanonicalValue {
        CanonicalValue::Record(vec![
            (1, CanonicalValue::U64(self.path_depth)),
            (2, CanonicalValue::U64(self.directory_entries)),
            (3, CanonicalValue::U64(self.selected_files)),
            (4, CanonicalValue::U64(self.file_bytes)),
            (5, CanonicalValue::U64(self.total_bytes)),
            (6, CanonicalValue::U64(self.graph_objects)),
            (7, CanonicalValue::U64(self.graph_references)),
            (8, CanonicalValue::U64(self.diagnostics)),
            (9, CanonicalValue::U64(self.source_nodes)),
            (10, CanonicalValue::U64(self.source_edges)),
            (11, CanonicalValue::U64(self.source_depth)),
            (12, CanonicalValue::U64(self.source_visited)),
        ])
    }

    /// Parses the four positive source-graph limits this configuration selects.
    ///
    /// # Errors
    ///
    /// Rejects a zero or oversized limit before any source graph is built.
    pub fn source_budgets(self) -> Result<SourceBudgets, RepositoryError> {
        let nodes = source_limit(self.source_nodes, RepositoryError::SourceNodesBudget)?;
        let edges = source_limit(self.source_edges, RepositoryError::SourceEdgesBudget)?;
        let depth = source_limit(self.source_depth, RepositoryError::SourceDepthBudget)?;
        let visited = source_limit(self.source_visited, RepositoryError::SourceVisitedBudget)?;
        Ok(SourceBudgets::new(
            SourceNodeBudget::new(nodes).map_err(|_| RepositoryError::SourceNodesBudget)?,
            SourceEdgeBudget::new(edges).map_err(|_| RepositoryError::SourceEdgesBudget)?,
            SourceDepthBudget::new(depth).map_err(|_| RepositoryError::SourceDepthBudget)?,
            SourceVisitedBudget::new(visited).map_err(|_| RepositoryError::SourceVisitedBudget)?,
        ))
    }

    fn validate(self) -> Result<(), RepositoryError> {
        if self.path_depth == 0 {
            return Err(RepositoryError::PathDepthBudget);
        }
        if self.path_depth > 256 {
            return Err(RepositoryError::PathDepthBudget);
        }
        if self.directory_entries == 0 {
            return Err(RepositoryError::DirectoryEntriesBudget);
        }
        if self.directory_entries > 1_000_000 {
            return Err(RepositoryError::DirectoryEntriesBudget);
        }
        if self.selected_files == 0 {
            return Err(RepositoryError::SelectedFilesBudget);
        }
        if self.selected_files > 100_000 {
            return Err(RepositoryError::SelectedFilesBudget);
        }
        if self.file_bytes == 0 {
            return Err(RepositoryError::FileBytesBudget);
        }
        if self.file_bytes > 268_435_456 {
            return Err(RepositoryError::FileBytesBudget);
        }
        if self.total_bytes == 0 {
            return Err(RepositoryError::TotalBytesBudget);
        }
        if self.total_bytes > 4_294_967_296 {
            return Err(RepositoryError::TotalBytesBudget);
        }
        if !(1..=1_000_000).contains(&self.graph_objects) {
            return Err(RepositoryError::GraphObjectsBudget);
        }
        if !(1..=10_000_000).contains(&self.graph_references) {
            return Err(RepositoryError::GraphReferencesBudget);
        }
        if !(1..=1_000_000).contains(&self.diagnostics) {
            return Err(RepositoryError::DiagnosticsBudget);
        }
        Ok(())
    }
}

const SOURCE_BUDGET_MAXIMUM: u32 = 1_000_000;

/// Narrows one configured source limit; the core newtype rejects a zero limit.
fn source_limit(value: u64, error: RepositoryError) -> Result<u32, RepositoryError> {
    u32::try_from(value)
        .ok()
        .filter(|limit| *limit <= SOURCE_BUDGET_MAXIMUM)
        .ok_or(error)
}

const fn default_graph_objects() -> u64 {
    200_000
}

const fn default_source_nodes() -> u64 {
    200_000
}

const fn default_source_edges() -> u64 {
    400_000
}

const fn default_source_depth() -> u64 {
    64
}

const fn default_source_visited() -> u64 {
    200_000
}

const fn default_graph_references() -> u64 {
    2_000_000
}

const fn default_diagnostics() -> u64 {
    100_000
}

/// One explicit path selector and adapter binding.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Selector {
    mount: Option<LogicalMountId>,
    root: RepositoryPath,
    root_identity: String,
    adapter: String,
    include: Vec<PortableGlob>,
    exclude: Vec<PortableGlob>,
    authority_expectation: AuthorityExpectation,
}

impl Selector {
    /// Selected mount for a version 3 configuration.
    #[must_use]
    pub const fn mount(&self) -> Option<&LogicalMountId> {
        self.mount.as_ref()
    }

    /// Canonical configured root text.
    #[must_use]
    pub fn root(&self) -> &str {
        &self.root_identity
    }

    /// Parsed configured root path.
    #[must_use]
    pub const fn root_path(&self) -> &RepositoryPath {
        &self.root
    }

    /// Exact adapter/schema identity.
    #[must_use]
    pub fn adapter(&self) -> &str {
        &self.adapter
    }

    /// Parsed authority expectation.
    #[must_use]
    pub const fn expectation(&self) -> AuthorityExpectation {
        self.authority_expectation
    }

    fn includes(&self, path: &RepositoryPath) -> bool {
        self.include.iter().any(|pattern| pattern.matches(path))
    }

    fn excludes(&self, path: &RepositoryPath) -> bool {
        self.exclude.iter().any(|pattern| pattern.matches(path))
    }

    pub(crate) fn selects_path(&self, path: &RepositoryPath) -> bool {
        let root = self.root_path().as_str();
        let in_root = path.as_str() == root
            || path
                .as_str()
                .strip_prefix(root)
                .is_some_and(|remainder| remainder.starts_with('/'));
        in_root && self.includes(path) && !self.excludes(path)
    }

    fn canonical_value(&self) -> CanonicalValue {
        let authority = match self.authority_expectation {
            AuthorityExpectation::Forbidden => 1,
            AuthorityExpectation::Optional => 2,
            AuthorityExpectation::Required => 3,
        };
        let root_identity = self.mount.as_ref().map_or_else(
            || self.root_identity.clone(),
            |_| self.root.as_str().to_owned(),
        );
        let mut fields = vec![
            (1, CanonicalValue::Text(root_identity)),
            (2, CanonicalValue::Text(self.adapter.clone())),
            (
                3,
                CanonicalValue::Set(
                    self.include
                        .iter()
                        .map(|pattern| CanonicalValue::Text(pattern.as_str().to_owned()))
                        .collect(),
                ),
            ),
            (
                4,
                CanonicalValue::Set(
                    self.exclude
                        .iter()
                        .map(|pattern| CanonicalValue::Text(pattern.as_str().to_owned()))
                        .collect(),
                ),
            ),
            (5, CanonicalValue::Enum(authority, None)),
        ];
        if let Some(mount) = &self.mount {
            fields.push((6, CanonicalValue::Text(mount.as_str().to_owned())));
        }
        CanonicalValue::Record(fields)
    }
}

/// One explicit effectful oracle invocation.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct OracleCommand {
    /// Stable oracle ID.
    pub id: String,
    /// Program path or one of the documented `builtin:*` test sentinels.
    pub program: String,
    /// Exact argument sequence.
    #[serde(default)]
    pub args: Vec<String>,
    /// Exact environment allowlist/value map after inherited environment clearing.
    #[serde(default)]
    pub environment: BTreeMap<String, String>,
    /// Closed working-root identity.
    #[serde(default = "default_working_root")]
    pub working_root: String,
    /// Optional bounded UTF-8 semantic stdin input.
    #[serde(default)]
    pub input: Option<String>,
    /// Effectful timeout; never included as elapsed canonical timing.
    pub timeout_ms: u64,
    /// Optional derivation relation ID when this is a consistency oracle.
    #[serde(default)]
    pub derivation: Option<String>,
    /// Maximum retained/read stdout bytes before normalized overflow.
    #[serde(default = "default_oracle_stream_limit")]
    pub stdout_limit: u64,
    /// Maximum retained/read stderr bytes before normalized overflow.
    #[serde(default = "default_oracle_stream_limit")]
    pub stderr_limit: u64,
    /// Maximum combined stdout/stderr bytes before normalized overflow.
    #[serde(default = "default_oracle_combined_limit")]
    pub combined_output_limit: u64,
    /// Maximum noncanonical bytes retained from each stream.
    #[serde(default = "default_raw_log_retention")]
    pub raw_log_retention: u64,
    /// Exact semantic result protocol identity.
    #[serde(default = "default_result_protocol")]
    pub result_protocol: String,
}

impl OracleCommand {
    pub(crate) fn canonical_value(&self) -> CanonicalValue {
        let environment = CanonicalValue::Set(
            self.environment
                .iter()
                .map(|(name, value)| {
                    CanonicalValue::Record(vec![
                        (1, CanonicalValue::Text(name.clone())),
                        (2, CanonicalValue::Text(value.clone())),
                    ])
                })
                .collect(),
        );
        let input = self.input.as_ref().map_or(CanonicalValue::None, |input| {
            CanonicalValue::Some(Box::new(CanonicalValue::Text(input.clone())))
        });
        let derivation = self
            .derivation
            .as_ref()
            .map_or(CanonicalValue::None, |derivation| {
                CanonicalValue::Some(Box::new(CanonicalValue::Text(derivation.clone())))
            });
        CanonicalValue::Record(vec![
            (1, CanonicalValue::Text(self.id.clone())),
            (2, CanonicalValue::Text(self.program.clone())),
            (
                3,
                CanonicalValue::Sequence(
                    self.args
                        .iter()
                        .cloned()
                        .map(CanonicalValue::Text)
                        .collect(),
                ),
            ),
            (4, environment),
            (5, CanonicalValue::Text(self.working_root.clone())),
            (6, input),
            (7, CanonicalValue::Text(self.result_protocol.clone())),
            (8, CanonicalValue::U64(self.stdout_limit)),
            (9, CanonicalValue::U64(self.stderr_limit)),
            (10, CanonicalValue::U64(self.combined_output_limit)),
            (11, CanonicalValue::U64(self.raw_log_retention)),
            (12, CanonicalValue::U64(self.timeout_ms)),
            (13, derivation),
            (14, CanonicalValue::None),
        ])
    }
}

const fn default_oracle_stream_limit() -> u64 {
    8_388_608
}

const fn default_oracle_combined_limit() -> u64 {
    16_777_216
}

const fn default_raw_log_retention() -> u64 {
    1_048_576
}

fn default_result_protocol() -> String {
    String::from("spec-provenance-result/v1")
}

fn default_working_root() -> String {
    String::from("repository-root")
}

/// One selected immutable file input.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SnapshotFile {
    /// Canonical repository-relative path.
    pub path: RepositoryPath,
    /// Exact raw bytes.
    pub bytes: Vec<u8>,
    /// Exact adapter/schema identity.
    pub adapter: String,
    /// Selector authority expectation.
    pub authority_expectation: AuthorityExpectation,
    /// Selector root used for selector-wide authority expectations.
    pub selector_root: String,
}

impl SnapshotFile {
    fn media_type(&self) -> Result<RawArtifactMediaType, RepositoryError> {
        match self.adapter.as_str() {
            "markdown/v1" | "openspec/v1" => Ok(RawArtifactMediaType::Markdown),
            "rust/v1" => Ok(RawArtifactMediaType::Rust),
            "binding/v1" => Ok(RawArtifactMediaType::Toml),
            "artifact/v1" => Ok(RawArtifactMediaType::OpaqueBytes),
            _ => Err(RepositoryError::UnsupportedAdapter),
        }
    }

    /// Returns the complete frozen snapshot-file canonical value.
    ///
    /// # Errors
    ///
    /// Rejects an unsupported adapter identity.
    pub fn canonical_value(&self) -> Result<CanonicalValue, RepositoryError> {
        repository_active_result(ActiveAlgorithm::build_default())
            .and_then(|active| self.canonical_value_with(active))
    }

    /// Returns the snapshot-file value for one active digest member.
    ///
    /// # Errors
    ///
    /// Rejects an unsupported adapter identity.
    pub fn canonical_value_with(
        &self,
        active: ActiveAlgorithm,
    ) -> Result<CanonicalValue, RepositoryError> {
        self.canonical_fields_with(active)
            .map(CanonicalValue::Record)
    }

    /// Returns the complete frozen snapshot-file semantic digest.
    ///
    /// # Errors
    ///
    /// Rejects an unsupported adapter or noncanonical retained value.
    pub fn semantic_digest(&self) -> Result<SemanticDigest, RepositoryError> {
        repository_active_result(ActiveAlgorithm::build_default())
            .and_then(|active| self.semantic_digest_with(active))
    }

    /// Returns the snapshot-file digest for one active digest member.
    ///
    /// # Errors
    ///
    /// Rejects an unsupported adapter or noncanonical retained value.
    pub fn semantic_digest_with(
        &self,
        active: ActiveAlgorithm,
    ) -> Result<SemanticDigest, RepositoryError> {
        semantic_digest_with(active, "snapshot-file", self.canonical_fields_with(active)?)
            .map_err(|_| RepositoryError::SnapshotEncoding)
    }

    fn canonical_fields_with(
        &self,
        active: ActiveAlgorithm,
    ) -> Result<Vec<(u16, CanonicalValue)>, RepositoryError> {
        let raw_digest =
            buoy_core::raw_artifact_digest_with(active, self.media_type()?, &self.bytes);
        Ok(vec![
            (1, CanonicalValue::Text(self.path.as_str().to_string())),
            (2, CanonicalValue::Enum(1, None)),
            (3, CanonicalValue::Digest(raw_digest)),
        ])
    }
}

fn canonical_value_set(
    values: impl IntoIterator<Item = CanonicalValue>,
) -> Result<CanonicalValue, RepositoryError> {
    let mut encoded = values
        .into_iter()
        .map(|value| {
            value
                .encode()
                .map(|bytes| (bytes, value))
                .map_err(|_| RepositoryError::SnapshotEncoding)
        })
        .collect::<Result<Vec<_>, _>>()?;
    encoded.sort_unstable_by(|left, right| left.0.cmp(&right.0));
    if encoded.windows(2).any(|pair| pair[0].0 == pair[1].0) {
        return Err(RepositoryError::SnapshotEncoding);
    }
    Ok(CanonicalValue::Set(
        encoded.into_iter().map(|(_, value)| value).collect(),
    ))
}

/// Exercises canonical snapshot-set encoding with explicit values.
#[cfg(feature = "coverage-controls")]
#[doc(hidden)]
pub fn canonical_value_set_control(
    values: Vec<CanonicalValue>,
) -> Result<CanonicalValue, RepositoryError> {
    canonical_value_set(values)
}

/// Explicit saved repository snapshot.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RepositorySnapshot {
    /// Selected files in canonical path order.
    pub files: Vec<SnapshotFile>,
}

impl RepositorySnapshot {
    /// Computes the exact frozen repository-snapshot identity from explicit inputs.
    ///
    /// # Errors
    ///
    /// Rejects unsupported adapters, duplicate canonical observations, or
    /// noncanonical nested values.
    pub fn semantic_digest(
        &self,
        repository_config: SemanticDigest,
        schema_tool_lock: SemanticDigest,
        rejected_entries: &[RejectedEntryObservation],
    ) -> Result<SemanticDigest, RepositoryError> {
        repository_active_result(ActiveAlgorithm::build_default()).and_then(|active| {
            self.semantic_digest_with(
                active,
                repository_config,
                schema_tool_lock,
                rejected_entries,
            )
        })
    }

    /// Computes the repository-snapshot identity for one active digest member.
    ///
    /// # Errors
    ///
    /// Rejects unsupported adapters, duplicate observations, or invalid nested values.
    pub fn semantic_digest_with(
        &self,
        active: ActiveAlgorithm,
        repository_config: SemanticDigest,
        schema_tool_lock: SemanticDigest,
        rejected_entries: &[RejectedEntryObservation],
    ) -> Result<SemanticDigest, RepositoryError> {
        let files = canonical_value_set(
            self.files
                .iter()
                .map(|file| file.canonical_value_with(active))
                .collect::<Result<Vec<_>, _>>()?,
        )?;
        let rejected = canonical_value_set(
            rejected_entries
                .iter()
                .map(RejectedEntryObservation::canonical_value),
        )?;
        semantic_digest_with(
            active,
            "repository-snapshot",
            vec![
                (1, CanonicalValue::Digest(repository_config)),
                (2, CanonicalValue::Digest(schema_tool_lock)),
                (3, files),
                (4, rejected),
            ],
        )
        .map_err(|_| RepositoryError::SnapshotEncoding)
    }

    /// Builds the explicit current artifact index from selected immutable bytes.
    ///
    /// # Errors
    ///
    /// Returns a stable artifact-index error when raw domain encoding or path
    /// uniqueness fails. Media/representation identity comes only from the
    /// closed adapter support matrix.
    pub fn artifact_index(&self) -> Result<ArtifactIndex, RepositoryError> {
        repository_active_result(ActiveAlgorithm::build_default())
            .and_then(|active| self.artifact_index_with(active))
    }

    /// Builds the artifact index for one active digest member.
    ///
    /// # Errors
    ///
    /// Returns an artifact-index error for invalid media or duplicate paths.
    pub fn artifact_index_with(
        &self,
        active: ActiveAlgorithm,
    ) -> Result<ArtifactIndex, RepositoryError> {
        let mut entries = Vec::with_capacity(self.files.len());
        for file in &self.files {
            let media_type = file.media_type()?;
            let digest = buoy_core::raw_artifact_digest_with(active, media_type, &file.bytes);
            entries.push(ArtifactEntry::saved_bytes(
                file.path.clone(),
                media_type,
                digest,
            ));
        }
        ArtifactIndex::new(entries).map_err(|_| RepositoryError::ArtifactIndex)
    }
}

/// Stable shell-side repository failure category.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RepositoryError {
    /// Configuration bytes are not UTF-8.
    ConfigUtf8,
    /// Configuration is not strict valid TOML.
    ConfigSyntax,
    /// Configuration bytes exceed the frozen parser input budget.
    ConfigBytesBudget,
    /// Schema identity or a closed field value is invalid.
    ConfigSchema,
    /// A logical mount ID violates the closed grammar.
    MountId,
    /// Two logical mount declarations use one ID.
    MountIdDuplicate,
    /// A logical mount prefix violates repository path policy.
    MountPrefix,
    /// Two logical mount prefixes overlap canonically or portably.
    MountPrefixOverlap,
    /// A selector omits or names an unknown logical mount.
    SelectorMount,
    /// One `ID=PATH` physical mapping is malformed.
    MountMapping,
    /// One invocation supplies a mount mapping more than once.
    MountMappingDuplicate,
    /// One declared logical mount lacks a physical mapping.
    MountMissing,
    /// One physical mapping has no logical mount declaration.
    MountExtra,
    /// A version 2 root and version 3 mount inputs were mixed.
    RootMountMixed,
    /// A physical mount root is absent or link-like.
    MountRoot,
    /// Two logical mounts resolved to one physical descriptor identity.
    ///
    /// Distinct logical mounts must name distinct physical locations: when
    /// two collapse onto one directory, mount-qualified source and output
    /// paths stop being distinguishable and a containment guard can be
    /// satisfied by a write that still lands on a selected source.
    MountRootAlias,
    /// Two mounted inputs project to one logical path.
    LogicalPathCollision,
    /// An artifact selector requests an authority role.
    ArtifactAuthority,
    /// Artifact configuration requests unsupported interpretation.
    ArtifactConfiguration,
    /// The configuration declares no active digest member.
    DigestActiveMissing,
    /// The configuration declares more than one active digest member.
    DigestActiveMultiple,
    /// The configuration names an unregistered digest member.
    DigestAlgorithmUnknown,
    /// The active digest member is absent from this build.
    DigestAlgorithmUnavailable,
    /// A selector root is absent or not safely classifiable.
    SelectorRoot,
    /// A selected link-like path was observed.
    LinkEntry,
    /// A selected submodule or nested repository link was observed.
    SubmoduleEntry,
    /// A path violates core normalization policy.
    PathPolicy,
    /// Two selectors overlap or canonical/portable paths collide.
    SelectionCollision,
    /// A selected file could not be read.
    ReadFailure,
    /// Adapter identity is unsupported.
    UnsupportedAdapter,
    /// Explicit current artifact index could not be constructed.
    ArtifactIndex,
    /// Snapshot canonical values could not be encoded exactly.
    SnapshotEncoding,
    /// A configured portable glob is outside the closed grammar.
    GlobSyntax,
    /// Path depth exceeds the configured budget.
    PathDepthBudget,
    /// Observed directory entries exceed the configured budget.
    DirectoryEntriesBudget,
    /// Selected file count exceeds the configured budget.
    SelectedFilesBudget,
    /// One selected file exceeds the configured byte budget.
    FileBytesBudget,
    /// Aggregate selected bytes exceed the configured budget.
    TotalBytesBudget,
    /// Materialized graph object count exceeds the configured budget.
    GraphObjectsBudget,
    /// Materialized direct-reference count exceeds the configured budget.
    GraphReferencesBudget,
    /// Stable diagnostic retention exceeds the configured budget.
    DiagnosticsBudget,
    /// The configured source-node limit is zero or oversized.
    SourceNodesBudget,
    /// The configured source-edge limit is zero or oversized.
    SourceEdgesBudget,
    /// The configured source-depth limit is zero or oversized.
    SourceDepthBudget,
    /// The configured visited-node limit is zero or oversized.
    SourceVisitedBudget,
    /// Compact binding selection names a change-detection digest member.
    BindingDigestClass,
    /// Oracle timeout is zero or exceeds the configured upper bound.
    OracleTimeoutBudget,
    /// Oracle stream/combined/log output budget is invalid.
    OracleOutputBudget,
    /// Oracle semantic result protocol identity is unsupported.
    OracleProtocol,
    /// An oracle names no declared working mount.
    OracleMount,
    /// The selected production platform has a reviewed blocking disposition.
    PlatformBlocker,
    /// Oracle concurrency is zero or exceeds the configured upper bound.
    OracleConcurrencyBudget,
}

impl RepositoryError {
    /// Stable machine diagnostic code.
    #[must_use]
    pub const fn code(self) -> &'static str {
        match self {
            Self::ConfigUtf8 => "REPO-CONFIG-UTF8",
            Self::ConfigSyntax => "REPO-CONFIG-SYNTAX",
            Self::ConfigBytesBudget => "REH-BUDGET-CONFIG-BYTES",
            Self::ConfigSchema => "REPO-CONFIG-SCHEMA",
            Self::MountId => "LSM-MOUNT-ID",
            Self::MountIdDuplicate => "LSM-MOUNT-ID-DUPLICATE",
            Self::MountPrefix => "LSM-MOUNT-PREFIX",
            Self::MountPrefixOverlap => "LSM-MOUNT-PREFIX-OVERLAP",
            Self::SelectorMount => "LSM-SELECTOR-MOUNT",
            Self::MountMapping => "LSM-MOUNT-MAPPING",
            Self::MountMappingDuplicate => "LSM-MOUNT-MAPPING-DUPLICATE",
            Self::MountMissing => "LSM-MOUNT-MISSING",
            Self::MountExtra => "LSM-MOUNT-EXTRA",
            Self::RootMountMixed => "LSM-ROOT-MOUNT-MIXED",
            Self::MountRoot => "REH-MOUNT-ROOT",
            Self::MountRootAlias => "LSM-MOUNT-ROOT-ALIAS",
            Self::LogicalPathCollision => "LSM-LOGICAL-PATH-COLLISION",
            Self::ArtifactAuthority => "RAS-ARTIFACT-AUTHORITY",
            Self::ArtifactConfiguration => "RAS-ARTIFACT-CONFIGURATION",
            Self::DigestActiveMissing => "DIG-ACTIVE-MISSING",
            Self::DigestActiveMultiple => "DIG-ACTIVE-MULTIPLE",
            Self::DigestAlgorithmUnknown => "DIG-ALGORITHM-UNKNOWN",
            Self::DigestAlgorithmUnavailable => "DIG-ALGORITHM-UNAVAILABLE",
            Self::SelectorRoot => "REPO-SELECTOR-ROOT",
            Self::LinkEntry => "DET-PATH-LINK",
            Self::SubmoduleEntry => "DET-PATH-SUBMODULE",
            Self::PathPolicy => "DET-PATH-POLICY",
            Self::SelectionCollision => "DET-PATH-SELECTION-COLLISION",
            Self::ReadFailure => "REPO-READ",
            Self::UnsupportedAdapter => "SPG-ADAPTER-UNSUPPORTED",
            Self::ArtifactIndex => "SPG-DERIVATION-ARTIFACT-INDEX",
            Self::SnapshotEncoding => "DET-ENC-SNAPSHOT",
            Self::GlobSyntax => "DET-PATH-GLOB-SYNTAX",
            Self::PathDepthBudget => "REH-BUDGET-PATH-DEPTH",
            Self::DirectoryEntriesBudget => "REH-BUDGET-DIRECTORY-ENTRIES",
            Self::SelectedFilesBudget => "REH-BUDGET-SELECTED-FILES",
            Self::FileBytesBudget => "REH-BUDGET-FILE-BYTES",
            Self::TotalBytesBudget => "REH-BUDGET-TOTAL-BYTES",
            Self::GraphObjectsBudget => "REH-BUDGET-GRAPH-OBJECTS",
            Self::GraphReferencesBudget => "REH-BUDGET-GRAPH-REFERENCES",
            Self::DiagnosticsBudget => "REH-BUDGET-DIAGNOSTICS",
            Self::SourceNodesBudget => "SRG-BUDGET-NODES",
            Self::SourceEdgesBudget => "SRG-BUDGET-EDGES",
            Self::SourceDepthBudget => "SRG-BUDGET-DEPTH",
            Self::SourceVisitedBudget => "SRG-BUDGET-VISITED",
            Self::BindingDigestClass => "CAB-DIGEST-CLASS",
            Self::OracleTimeoutBudget => "REH-BUDGET-ORACLE-TIMEOUT",
            Self::OracleOutputBudget => "REH-BUDGET-ORACLE-OUTPUT",
            Self::OracleProtocol => "EVD-EXEC-PROTOCOL-SCHEMA",
            Self::OracleMount => "REH-ORACLE-MOUNT",
            Self::PlatformBlocker => "REH-PLATFORM-WINDOWS-SNAPSHOT-BLOCKED",
            Self::OracleConcurrencyBudget => "REH-BUDGET-ORACLE-CONCURRENCY",
        }
    }
}

fn repository_active_result<T>(
    result: Result<T, ActiveAlgorithmError>,
) -> Result<T, RepositoryError> {
    result.map_err(|_| RepositoryError::DigestAlgorithmUnavailable)
}

/// Runs the repository active-algorithm error translation for coverage controls.
///
/// # Errors
///
/// Always returns the stable unavailable-algorithm error.
#[cfg(feature = "coverage-controls")]
#[doc(hidden)]
pub fn repository_active_error_control() -> Result<ActiveAlgorithm, RepositoryError> {
    repository_active_result(Err(ActiveAlgorithmError::Unavailable(
        DigestAlgorithm::Xxh3_128,
    )))
}

/// Injects an unavailable adapter through complete repository configuration parsing.
///
/// # Errors
///
/// Always returns the stable unavailable-algorithm error for valid configuration input.
#[cfg(feature = "coverage-controls")]
#[doc(hidden)]
pub fn repository_config_unavailable_control(
    bytes: &[u8],
) -> Result<RepositoryConfig, RepositoryError> {
    RepositoryConfig::parse_internal(bytes, true)
}

/// Loads strict repository configuration from one explicit path.
///
/// # Errors
///
/// Returns a stable read or configuration error.
pub fn load_config(path: &Path) -> Result<RepositoryConfig, RepositoryError> {
    let file = fs::File::open(path).map_err(|_| RepositoryError::ReadFailure)?;
    let mut bytes = Vec::new();
    file.take(CONFIGURATION_BYTES_MAXIMUM as u64 + 1)
        .read_to_end(&mut bytes)
        .map_err(|_| RepositoryError::ReadFailure)?;
    if bytes.len() > CONFIGURATION_BYTES_MAXIMUM {
        return Err(RepositoryError::ConfigBytesBudget);
    }
    RepositoryConfig::parse(&bytes)
}

/// Complete parsed physical mappings for one logical mount invocation.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MountMappings {
    roots: BTreeMap<LogicalMountId, PathBuf>,
}

impl MountMappings {
    /// Parses repeated `ID=PATH` command values into one complete mapping type.
    ///
    /// # Errors
    ///
    /// Returns a stable mapping error for malformed or duplicate mount values.
    pub fn parse(values: &[String]) -> Result<Self, RepositoryError> {
        let mut roots = BTreeMap::new();
        for value in values {
            let (id, path) = value
                .split_once('=')
                .filter(|(id, path)| !id.is_empty() && !path.is_empty())
                .ok_or(RepositoryError::MountMapping)?;
            let id = LogicalMountId::new(id).map_err(|_| RepositoryError::MountMapping)?;
            if roots.insert(id, PathBuf::from(path)).is_some() {
                return Err(RepositoryError::MountMappingDuplicate);
            }
        }
        Ok(Self { roots })
    }

    /// Constructs mappings from already-parsed IDs and physical paths.
    ///
    /// # Errors
    ///
    /// Returns a duplicate-mapping error when one ID occurs more than once.
    pub fn new(values: Vec<(LogicalMountId, PathBuf)>) -> Result<Self, RepositoryError> {
        let mut roots = BTreeMap::new();
        for (id, path) in values {
            if roots.insert(id, path).is_some() {
                return Err(RepositoryError::MountMappingDuplicate);
            }
        }
        Ok(Self { roots })
    }

    /// Returns the physical root for one parsed mount ID.
    #[must_use]
    pub fn root(&self, id: &LogicalMountId) -> Option<&Path> {
        self.roots.get(id).map(PathBuf::as_path)
    }

    /// Validates exact completeness against one version 3 configuration.
    ///
    /// # Errors
    ///
    /// Returns a missing or extra mapping error before any mount root opens.
    pub fn validate(&self, config: &RepositoryConfig) -> Result<(), RepositoryError> {
        if !config.is_mounted() {
            return Err(RepositoryError::RootMountMixed);
        }
        let declared = config
            .mounts()
            .iter()
            .map(LogicalMount::id)
            .collect::<BTreeSet<_>>();
        let supplied = self.roots.keys().collect::<BTreeSet<_>>();
        if declared.iter().any(|id| !supplied.contains(id)) {
            return Err(RepositoryError::MountMissing);
        }
        if supplied.iter().any(|id| !declared.contains(id)) {
            return Err(RepositoryError::MountExtra);
        }
        Ok(())
    }
}

/// Constructs a descriptor-relative Unix snapshot without consulting Git,
/// ignore files, provider state, or path re-resolution after opening the root.
///
/// # Errors
///
/// Returns a stable error for invalid selectors, paths, links, overlaps,
/// unsupported adapters, budgets, or unreadable selected entries.
#[cfg(unix)]
pub fn snapshot_repository(
    root: &Path,
    config: &RepositoryConfig,
) -> Result<RepositorySnapshot, RepositoryError> {
    if config.is_mounted() {
        return Err(RepositoryError::RootMountMixed);
    }
    snapshot_repository_after_roots_open(&[(None, root)], config, &mut || {}, None)
}

/// Constructs one joined Unix snapshot from complete logical mount mappings.
///
/// # Errors
///
/// Returns a stable error for incomplete mappings, mount roots, paths, links,
/// overlaps, unsupported adapters, budgets, or unreadable selected entries.
#[cfg(unix)]
#[allow(clippy::expect_used, clippy::missing_panics_doc)]
pub fn snapshot_mounted_repository(
    mappings: &MountMappings,
    config: &RepositoryConfig,
) -> Result<RepositorySnapshot, RepositoryError> {
    mappings.validate(config)?;
    let roots = config
        .mounts()
        .iter()
        .map(|mount| {
            (
                Some(mount),
                mappings
                    .root(mount.id())
                    .expect("validated mappings contain every declared mount"),
            )
        })
        .collect::<Vec<_>>();
    snapshot_repository_after_roots_open(&roots, config, &mut || {}, None)
}

#[cfg(unix)]
#[allow(
    clippy::cast_possible_truncation,
    clippy::cast_possible_wrap,
    clippy::too_many_lines
)]
fn snapshot_repository_after_roots_open(
    roots: &[(Option<&LogicalMount>, &Path)],
    config: &RepositoryConfig,
    after_roots_open: &mut dyn FnMut(),
    fault: Option<&str>,
) -> Result<RepositorySnapshot, RepositoryError> {
    let mut opened = Vec::with_capacity(roots.len());
    // Round-5 finding H5: two logical mounts that resolve to ONE physical
    // location silently collapse the source/output separation the mount
    // model exists to provide -- a path "in the project mount" and the same
    // path "in the authority mount" become the same bytes, so a guard that
    // proves an output lies outside every selected source can be satisfied
    // while the write still lands on a selected source.
    //
    // `MountMappings` only ever deduplicated by mount ID, and comparing the
    // supplied PATH STRINGS is not sufficient: a symlinked root, a `..`
    // spelling, a trailing slash, or a case-insensitive filesystem all
    // produce distinct strings for one physical directory. Identity is
    // therefore taken from the RETAINED DESCRIPTOR after the no-follow open
    // -- the same descriptor every later operation uses -- so this observes
    // exactly the location that will be traversed and cannot be defeated by
    // how the caller spelled it.
    let mut identities = Vec::with_capacity(roots.len());
    for (mount, root) in roots {
        let root_error = || {
            if mount.is_some() {
                RepositoryError::MountRoot
            } else {
                RepositoryError::SelectorRoot
            }
        };
        let descriptor = open_directory(root).map_err(|_| root_error())?;
        let status = if fault == Some("opened-root-stat") {
            Err(rustix::io::Errno::IO)
        } else {
            rustix::fs::fstat(&descriptor)
        }
        .map_err(|_| root_error())?;
        // Only equality matters here, so the native `st_dev`/`st_ino` types
        // are compared directly. Casting them to a common width would add
        // nothing but a sign-loss lint: two descriptors on one directory
        // report one pair, and that is the entire property being tested.
        let identity = (status.st_dev, status.st_ino);
        if identities.contains(&identity) {
            return Err(RepositoryError::MountRootAlias);
        }
        identities.push(identity);
        opened.push((*mount, descriptor));
    }
    after_roots_open();
    let budgets = config.budgets();
    let mut directory_entries = 0_u64;
    let mut selected_bytes = 0_u64;
    let mut selected = BTreeMap::<RepositoryPath, SnapshotFile>::new();
    for (mount, root_fd) in &opened {
        for selector in config
            .selectors()
            .iter()
            .filter(|selector| selector.mount() == mount.map(LogicalMount::id))
        {
            if !matches!(
                selector.adapter(),
                "artifact/v1" | "binding/v1" | "markdown/v1" | "openspec/v1" | "rust/v1"
            ) {
                return Err(RepositoryError::UnsupportedAdapter);
            }
            let selector_root = if fault == Some("selector-path") {
                RepositoryPath::new("/").map_err(|_| RepositoryError::PathPolicy)?
            } else {
                selector.root_path().clone()
            };
            if selector_root.as_str().split('/').next_back() == Some(".git") {
                return Err(RepositoryError::SubmoduleEntry);
            }
            let selector_fd = open_relative_entry(root_fd, selector_root.as_str(), fault)?;
            let depth_value = if fault == Some("selector-depth") {
                -1_i64
            } else {
                let prefix_depth =
                    mount.map_or(0, |mount| mount.prefix().as_str().split('/').count());
                (prefix_depth + selector_root.as_str().split('/').count()) as i64
            };
            let selector_depth =
                u64::try_from(depth_value).map_err(|_| RepositoryError::PathDepthBudget)?;
            if selector_depth > budgets.path_depth {
                return Err(RepositoryError::PathDepthBudget);
            }
            let selector_stat = if fault == Some("selector-stat") {
                Err(rustix::io::Errno::IO)
            } else {
                fstat(&selector_fd)
            }
            .map_err(|_| RepositoryError::ReadFailure)?;
            let selector_type = if fault == Some("selector-type") {
                FileType::Symlink
            } else {
                FileType::from_raw_mode(selector_stat.st_mode)
            };
            let mut pending = Vec::new();
            match selector_type {
                FileType::Directory => pending.push((
                    selector_fd,
                    selector_root.as_str().to_owned(),
                    selector_depth,
                )),
                FileType::RegularFile => {
                    if mount.is_none() {
                        return Err(RepositoryError::SelectorRoot);
                    }
                    let observed_value = if fault == Some("observed-size") {
                        -1_i128
                    } else {
                        i128::from(selector_stat.st_size)
                    };
                    retain_selected_file(
                        selector,
                        *mount,
                        selector_root.clone(),
                        selector_fd,
                        observed_value,
                        budgets,
                        &mut selected_bytes,
                        &mut selected,
                        fault,
                    )?;
                }
                _ => return Err(RepositoryError::LinkEntry),
            }
            while let Some((directory_fd, directory_path, depth)) = pending.pop() {
                let directory_result = if fault == Some("directory-read") {
                    Err(rustix::io::Errno::IO)
                } else {
                    Dir::read_from(&directory_fd)
                };
                let mut directory = directory_result.map_err(|_| RepositoryError::ReadFailure)?;
                let mut names = Vec::new();
                while let Some(entry) = directory.read() {
                    let entry_result = if fault == Some("directory-entry") {
                        Err(rustix::io::Errno::IO)
                    } else {
                        entry
                    };
                    let entry = entry_result.map_err(|_| RepositoryError::ReadFailure)?;
                    let name = entry.file_name();
                    if name.to_bytes() == b"." || name.to_bytes() == b".." {
                        continue;
                    }
                    directory_entries += 1;
                    if directory_entries > budgets.directory_entries {
                        return Err(RepositoryError::DirectoryEntriesBudget);
                    }
                    names.push(name.to_owned());
                }
                names.sort_unstable_by(|left, right| left.to_bytes().cmp(right.to_bytes()));
                let mut child_directories = Vec::new();
                for name in names {
                    let name_text = if fault == Some("entry-path") {
                        None
                    } else {
                        name.to_str().ok()
                    }
                    .ok_or(RepositoryError::PathPolicy)?;
                    if name_text == ".git" {
                        return Err(RepositoryError::SubmoduleEntry);
                    }
                    let child_path = if fault == Some("child-path") {
                        String::from("/")
                    } else {
                        format!("{directory_path}/{name_text}")
                    };
                    let child_fd = openat(
                        &directory_fd,
                        &name,
                        OFlags::RDONLY | OFlags::CLOEXEC | OFlags::NOFOLLOW | OFlags::NONBLOCK,
                        Mode::empty(),
                    )
                    .map_err(|_| RepositoryError::LinkEntry)?;
                    let stat_result = if fault == Some("stat") {
                        Err(rustix::io::Errno::IO)
                    } else {
                        fstat(&child_fd)
                    };
                    let stat = stat_result.map_err(|_| RepositoryError::ReadFailure)?;
                    let file_type = if fault == Some("file-type") {
                        FileType::Fifo
                    } else {
                        FileType::from_raw_mode(stat.st_mode)
                    };
                    match file_type {
                        FileType::Directory => {
                            let next_depth = depth + 1;
                            if next_depth > budgets.path_depth {
                                return Err(RepositoryError::PathDepthBudget);
                            }
                            child_directories.push((child_fd, child_path, next_depth));
                        }
                        FileType::RegularFile => {
                            let mount_relative = RepositoryPath::new(&child_path)
                                .map_err(|_| RepositoryError::PathPolicy)?;
                            let observed_value = if fault == Some("observed-size") {
                                -1_i128
                            } else {
                                i128::from(stat.st_size)
                            };
                            retain_selected_file(
                                selector,
                                *mount,
                                mount_relative,
                                child_fd,
                                observed_value,
                                budgets,
                                &mut selected_bytes,
                                &mut selected,
                                fault,
                            )?;
                        }
                        _ => return Err(RepositoryError::LinkEntry),
                    }
                }
                pending.extend(child_directories.into_iter().rev());
            }
        }
    }
    let selected_paths = selected.keys().cloned().collect();
    #[cfg(feature = "coverage-controls")]
    let selected_paths = if fault == Some("selected-paths") {
        vec![
            RepositoryPath::reviewed_control("spec/Straße.md"),
            RepositoryPath::reviewed_control("spec/STRASSE.md"),
        ]
    } else {
        selected_paths
    };
    if validate_selected_paths(selected_paths).is_err() {
        return Err(if config.is_mounted() {
            RepositoryError::LogicalPathCollision
        } else {
            RepositoryError::SelectionCollision
        });
    }
    Ok(RepositorySnapshot {
        files: selected.into_values().collect(),
    })
}

#[cfg(unix)]
#[allow(
    clippy::cast_possible_truncation,
    clippy::cast_possible_wrap,
    clippy::too_many_arguments
)]
#[allow(clippy::expect_used)]
fn retain_selected_file(
    selector: &Selector,
    mount: Option<&LogicalMount>,
    mount_relative: RepositoryPath,
    child_fd: OwnedFd,
    observed_value: i128,
    budgets: SnapshotBudgets,
    selected_bytes: &mut u64,
    selected: &mut BTreeMap<RepositoryPath, SnapshotFile>,
    fault: Option<&str>,
) -> Result<(), RepositoryError> {
    if matches!(fault, Some("include-pattern" | "exclude-pattern")) {
        return Err(RepositoryError::GlobSyntax);
    }
    if !selector.includes(&mount_relative) || selector.excludes(&mount_relative) {
        return Ok(());
    }
    let path = if let Some(mount) = mount {
        project_logical_path(mount, &mount_relative)
            .expect("parsed mount and relative paths always project")
    } else {
        mount_relative
    };
    let selected_count = selected.len() as u64 + 1;
    if selected_count > budgets.selected_files {
        return Err(RepositoryError::SelectedFilesBudget);
    }
    let observed_size =
        u64::try_from(observed_value).map_err(|_| RepositoryError::FileBytesBudget)?;
    if observed_size > budgets.file_bytes {
        return Err(RepositoryError::FileBytesBudget);
    }
    if selected_bytes
        .checked_add(observed_size)
        .is_none_or(|total| total > budgets.total_bytes)
    {
        return Err(RepositoryError::TotalBytesBudget);
    }
    let capacity = observed_size as usize;
    let mut file = std::fs::File::from(child_fd);
    let bytes = if fault == Some("file-read") {
        Err(RepositoryError::ReadFailure)
    } else {
        read_bounded(&mut file, budgets.file_bytes, capacity)
    }?;
    let actual_size = bytes.len() as u64;
    if fault == Some("actual-total") {
        *selected_bytes = budgets.total_bytes;
    }
    *selected_bytes += actual_size;
    if *selected_bytes > budgets.total_bytes {
        return Err(RepositoryError::TotalBytesBudget);
    }
    let selector_root = if let Some(mount) = mount {
        project_logical_path(mount, &selector.root)
            .expect("parsed mount and selector paths always project")
            .as_str()
            .to_owned()
    } else {
        selector.root().to_owned()
    };
    let file = SnapshotFile {
        path: path.clone(),
        bytes,
        adapter: selector.adapter().to_owned(),
        authority_expectation: if fault == Some("expectation") {
            return Err(RepositoryError::ConfigSchema);
        } else {
            selector.expectation()
        },
        selector_root,
    };
    if selected.insert(path, file).is_some() {
        return Err(if mount.is_some() {
            RepositoryError::LogicalPathCollision
        } else {
            RepositoryError::SelectionCollision
        });
    }
    Ok(())
}

fn validate_selected_paths(paths: Vec<RepositoryPath>) -> Result<(), RepositoryError> {
    canonical_paths(paths)
        .map(|_| ())
        .map_err(|_| RepositoryError::SelectionCollision)
}

/// Runs deterministic selected-path collision controls without filesystem case-fold behavior.
#[cfg(feature = "coverage-controls")]
#[doc(hidden)]
pub fn validate_selected_paths_control(paths: Vec<RepositoryPath>) -> Result<(), RepositoryError> {
    validate_selected_paths(paths)
}

#[cfg(unix)]
fn read_bounded(
    reader: &mut dyn std::io::Read,
    maximum: u64,
    capacity: usize,
) -> Result<Vec<u8>, RepositoryError> {
    let limit = maximum + 1;
    let mut bytes = Vec::with_capacity(capacity);
    reader
        .take(limit)
        .read_to_end(&mut bytes)
        .map_err(|_| RepositoryError::ReadFailure)?;
    if bytes.len() as u64 > maximum {
        return Err(RepositoryError::FileBytesBudget);
    }
    Ok(bytes)
}

/// Runs bounded-reader controls with deterministic injected readers.
#[cfg(all(feature = "coverage-controls", unix))]
#[doc(hidden)]
pub fn read_bounded_control(
    reader: &mut dyn std::io::Read,
    maximum: u64,
    capacity: usize,
) -> Result<Vec<u8>, RepositoryError> {
    read_bounded(reader, maximum, capacity)
}

/// Runs a descriptor-root replacement control after the root descriptor opens.
#[cfg(all(feature = "coverage-controls", unix))]
#[doc(hidden)]
pub fn snapshot_repository_after_root_open_control<F: FnMut()>(
    root: &Path,
    config: &RepositoryConfig,
    mut after_root_open: F,
) -> Result<RepositorySnapshot, RepositoryError> {
    snapshot_repository_after_root_open_control_dyn(root, config, &mut after_root_open)
}

#[cfg(all(feature = "coverage-controls", unix))]
fn snapshot_repository_after_root_open_control_dyn(
    root: &Path,
    config: &RepositoryConfig,
    after_root_open: &mut dyn FnMut(),
) -> Result<RepositorySnapshot, RepositoryError> {
    if config.is_mounted() {
        return Err(RepositoryError::RootMountMixed);
    }
    snapshot_repository_after_roots_open(&[(None, root)], config, after_root_open, None)
}

/// Runs one mounted root-replacement control after every descriptor opens.
///
/// # Errors
///
/// Returns the same stable errors as [`snapshot_mounted_repository`].
#[cfg(all(feature = "coverage-controls", unix))]
#[doc(hidden)]
#[allow(clippy::expect_used, clippy::missing_panics_doc)]
pub fn snapshot_mounted_repository_after_roots_open_control<F: FnMut()>(
    mappings: &MountMappings,
    config: &RepositoryConfig,
    mut after_roots_open: F,
) -> Result<RepositorySnapshot, RepositoryError> {
    snapshot_mounted_repository_after_roots_open_control_dyn(
        mappings,
        config,
        &mut after_roots_open,
    )
}

#[cfg(all(feature = "coverage-controls", unix))]
#[allow(clippy::expect_used)]
fn snapshot_mounted_repository_after_roots_open_control_dyn(
    mappings: &MountMappings,
    config: &RepositoryConfig,
    after_roots_open: &mut dyn FnMut(),
) -> Result<RepositorySnapshot, RepositoryError> {
    mappings.validate(config)?;
    let roots = config
        .mounts()
        .iter()
        .map(|mount| {
            (
                Some(mount),
                mappings
                    .root(mount.id())
                    .expect("validated mappings contain every declared mount"),
            )
        })
        .collect::<Vec<_>>();
    snapshot_repository_after_roots_open(&roots, config, after_roots_open, None)
}

/// Injects one reviewed failure into a complete mounted snapshot.
#[cfg(all(feature = "coverage-controls", unix))]
#[doc(hidden)]
#[allow(clippy::expect_used, clippy::missing_panics_doc)]
pub fn snapshot_mounted_repository_fault_control(
    mappings: &MountMappings,
    config: &RepositoryConfig,
    fault: &str,
) -> Result<RepositorySnapshot, RepositoryError> {
    mappings.validate(config)?;
    let roots = config
        .mounts()
        .iter()
        .map(|mount| {
            (
                Some(mount),
                mappings
                    .root(mount.id())
                    .expect("validated mappings contain every declared mount"),
            )
        })
        .collect::<Vec<_>>();
    snapshot_repository_after_roots_open(&roots, config, &mut || {}, Some(fault))
}

/// Injects one reviewed descriptor-snapshot failure at its owning operation.
#[cfg(all(feature = "coverage-controls", unix))]
#[doc(hidden)]
pub fn snapshot_repository_fault_control(
    root: &Path,
    config: &RepositoryConfig,
    fault: &str,
) -> Result<RepositorySnapshot, RepositoryError> {
    if config.is_mounted() {
        return Err(RepositoryError::RootMountMixed);
    }
    snapshot_repository_after_roots_open(&[(None, root)], config, &mut || {}, Some(fault))
}

#[cfg(unix)]
fn open_directory(path: &Path) -> rustix::io::Result<OwnedFd> {
    open(
        path,
        OFlags::RDONLY | OFlags::CLOEXEC | OFlags::NOFOLLOW | OFlags::DIRECTORY,
        Mode::empty(),
    )
}

#[cfg(unix)]
fn open_relative_entry(
    root: &OwnedFd,
    relative: &str,
    fault: Option<&str>,
) -> Result<OwnedFd, RepositoryError> {
    let initial = openat(
        root,
        ".",
        OFlags::RDONLY | OFlags::CLOEXEC | OFlags::NOFOLLOW | OFlags::DIRECTORY,
        Mode::empty(),
    );
    let initial = if fault == Some("relative-root") {
        Err(rustix::io::Errno::IO)
    } else {
        initial
    };
    let mut current = initial.map_err(|_| RepositoryError::SelectorRoot)?;
    let components = relative.split('/').collect::<Vec<_>>();
    for (index, component) in components.iter().enumerate() {
        let mut flags = OFlags::RDONLY | OFlags::CLOEXEC | OFlags::NOFOLLOW | OFlags::NONBLOCK;
        if index + 1 != components.len() {
            flags |= OFlags::DIRECTORY;
        }
        let opened = openat(&current, *component, flags, Mode::empty());
        let opened = if fault == Some("relative-component") {
            Err(rustix::io::Errno::IO)
        } else {
            opened
        };
        current = opened.map_err(|_| RepositoryError::SelectorRoot)?;
    }
    Ok(current)
}

/// Reviewed blocker identity for the unavailable Windows handle-relative backend.
pub const WINDOWS_SNAPSHOT_BLOCKER_ID: &str = "PLATFORM-WINDOWS-SNAPSHOT-001";

/// Returns the exact reviewed Windows production-platform blocker disposition.
#[must_use]
pub const fn windows_snapshot_blocker_disposition() -> (&'static str, RepositoryError) {
    (
        WINDOWS_SNAPSHOT_BLOCKER_ID,
        RepositoryError::PlatformBlocker,
    )
}

/// Fails closed on Windows until the reviewed handle-relative/reparse-safe
/// backend and its native race fixtures discharge the production blocker.
///
/// # Errors
///
/// Always returns the stable reviewed production-platform blocker.
#[cfg(windows)]
pub fn snapshot_repository(
    _root: &Path,
    _config: &RepositoryConfig,
) -> Result<RepositorySnapshot, RepositoryError> {
    Err(RepositoryError::PlatformBlocker)
}

/// Fails closed for mounted snapshots without the Unix descriptor backend.
///
/// # Errors
///
/// Returns a mapping error or the reviewed platform blocker.
#[cfg(not(unix))]
pub fn snapshot_mounted_repository(
    mappings: &MountMappings,
    config: &RepositoryConfig,
) -> Result<RepositorySnapshot, RepositoryError> {
    mappings.validate(config)?;
    Err(RepositoryError::PlatformBlocker)
}

#[cfg(any(windows, feature = "coverage-controls"))]
const fn windows_attributes_are_reparse(attributes: u32) -> bool {
    attributes & WINDOWS_FILE_ATTRIBUTE_REPARSE_POINT != 0
}

/// Exercises the exact Windows reparse-attribute classifier without host path semantics.
///
/// # Errors
///
/// Returns the stable link diagnostic when the Windows reparse-point bit is present.
#[cfg(feature = "coverage-controls")]
#[doc(hidden)]
pub fn windows_reparse_attributes_control(attributes: u32) -> Result<(), RepositoryError> {
    if windows_attributes_are_reparse(attributes) {
        Err(RepositoryError::LinkEntry)
    } else {
        Ok(())
    }
}

#[cfg(not(unix))]
fn metadata_is_link_like(metadata: &fs::Metadata) -> bool {
    if metadata.file_type().is_symlink() {
        return true;
    }
    #[cfg(windows)]
    {
        use std::os::windows::fs::MetadataExt as _;
        windows_attributes_are_reparse(metadata.file_attributes())
    }
    #[cfg(not(windows))]
    {
        false
    }
}

/// Constructs a fallback snapshot on non-Unix, non-Windows platforms.
/// This backend is not part of the declared production platform set.
///
/// # Errors
///
/// Returns a stable error for invalid selectors, paths, links, overlaps,
/// unsupported adapters, or unreadable selected entries.
#[cfg(not(any(unix, windows)))]
pub fn snapshot_repository(
    root: &Path,
    config: &RepositoryConfig,
) -> Result<RepositorySnapshot, RepositoryError> {
    if config.is_mounted() {
        return Err(RepositoryError::RootMountMixed);
    }
    let root_metadata = fs::symlink_metadata(root).map_err(|_| RepositoryError::SelectorRoot)?;
    if metadata_is_link_like(&root_metadata) {
        return Err(RepositoryError::LinkEntry);
    }
    let budgets = config.budgets();
    let mut directory_entries = 0_u64;
    let mut selected_bytes = 0_u64;
    let mut selected = BTreeMap::<RepositoryPath, SnapshotFile>::new();
    for selector in config.selectors() {
        if !matches!(
            selector.adapter(),
            "artifact/v1" | "binding/v1" | "markdown/v1" | "openspec/v1" | "rust/v1"
        ) {
            return Err(RepositoryError::UnsupportedAdapter);
        }
        let selector_root = selector.root_path().clone();
        let absolute = root.join(selector_root.as_str());
        let metadata =
            fs::symlink_metadata(&absolute).map_err(|_| RepositoryError::SelectorRoot)?;
        if metadata_is_link_like(&metadata) {
            return Err(RepositoryError::LinkEntry);
        }
        let mut paths = Vec::new();
        collect_regular_files(&absolute, budgets, &mut directory_entries, &mut paths)?;
        paths.sort_unstable();
        for absolute_file in paths {
            let relative = absolute_file
                .strip_prefix(root)
                .map_err(|_| RepositoryError::PathPolicy)?;
            let text = relative_path_text(relative)?;
            let path = RepositoryPath::new(&text).map_err(|_| RepositoryError::PathPolicy)?;
            if !matches_patterns(&path, &selector.include)
                || matches_patterns(&path, &selector.exclude)
            {
                continue;
            }
            let selected_count = u64::try_from(selected.len())
                .map_err(|_| RepositoryError::SelectedFilesBudget)?
                .checked_add(1)
                .ok_or(RepositoryError::SelectedFilesBudget)?;
            if selected_count > budgets.selected_files {
                return Err(RepositoryError::SelectedFilesBudget);
            }
            let observed_size = fs::metadata(&absolute_file)
                .map_err(|_| RepositoryError::ReadFailure)?
                .len();
            if observed_size > budgets.file_bytes {
                return Err(RepositoryError::FileBytesBudget);
            }
            let next_total = selected_bytes
                .checked_add(observed_size)
                .ok_or(RepositoryError::TotalBytesBudget)?;
            if next_total > budgets.total_bytes {
                return Err(RepositoryError::TotalBytesBudget);
            }
            let bytes = fs::read(&absolute_file).map_err(|_| RepositoryError::ReadFailure)?;
            let actual_size =
                u64::try_from(bytes.len()).map_err(|_| RepositoryError::FileBytesBudget)?;
            if actual_size > budgets.file_bytes {
                return Err(RepositoryError::FileBytesBudget);
            }
            selected_bytes = selected_bytes
                .checked_add(actual_size)
                .ok_or(RepositoryError::TotalBytesBudget)?;
            if selected_bytes > budgets.total_bytes {
                return Err(RepositoryError::TotalBytesBudget);
            }
            let file = SnapshotFile {
                path: path.clone(),
                bytes,
                adapter: selector.adapter().to_string(),
                authority_expectation: selector.expectation(),
                selector_root: selector.root().to_string(),
            };
            if selected.insert(path, file).is_some() {
                return Err(RepositoryError::SelectionCollision);
            }
        }
    }
    let paths = selected.keys().cloned().collect::<Vec<_>>();
    canonical_paths(paths).map_err(|_| RepositoryError::SelectionCollision)?;
    Ok(RepositorySnapshot {
        files: selected.into_values().collect(),
    })
}

#[cfg(not(any(unix, windows)))]
fn collect_regular_files(
    root: &Path,
    budgets: SnapshotBudgets,
    observed_entries: &mut u64,
    output: &mut Vec<PathBuf>,
) -> Result<(), RepositoryError> {
    let mut pending = vec![(root.to_path_buf(), 0_u64)];
    while let Some((path, depth)) = pending.pop() {
        if depth > budgets.path_depth {
            return Err(RepositoryError::PathDepthBudget);
        }
        let metadata = fs::symlink_metadata(&path).map_err(|_| RepositoryError::ReadFailure)?;
        if metadata_is_link_like(&metadata) {
            return Err(RepositoryError::LinkEntry);
        }
        if metadata.is_file() {
            if path.file_name().and_then(|name| name.to_str()) == Some(".git") {
                return Err(RepositoryError::SubmoduleEntry);
            }
            output.push(path);
            continue;
        }
        if !metadata.is_dir() {
            return Err(RepositoryError::LinkEntry);
        }
        let entries = fs::read_dir(&path).map_err(|_| RepositoryError::ReadFailure)?;
        let mut children = Vec::new();
        for entry in entries {
            *observed_entries = observed_entries
                .checked_add(1)
                .ok_or(RepositoryError::DirectoryEntriesBudget)?;
            if *observed_entries > budgets.directory_entries {
                return Err(RepositoryError::DirectoryEntriesBudget);
            }
            children.push(entry.map_err(|_| RepositoryError::ReadFailure)?.path());
        }
        children.sort_unstable();
        let next_depth = depth
            .checked_add(1)
            .ok_or(RepositoryError::PathDepthBudget)?;
        if !children.is_empty() && next_depth > budgets.path_depth {
            return Err(RepositoryError::PathDepthBudget);
        }
        pending.extend(children.into_iter().rev().map(|child| (child, next_depth)));
    }
    Ok(())
}

#[cfg(not(any(unix, windows)))]
fn relative_path_text(path: &Path) -> Result<String, RepositoryError> {
    let mut text = String::new();
    for component in path.components() {
        let segment = component
            .as_os_str()
            .to_str()
            .ok_or(RepositoryError::PathPolicy)?;
        if !text.is_empty() {
            text.push('/');
        }
        text.push_str(segment);
    }
    Ok(text)
}

#[cfg(not(any(unix, windows)))]
fn matches_patterns(path: &RepositoryPath, patterns: &[PortableGlob]) -> bool {
    patterns.iter().any(|pattern| pattern.matches(path))
}

#[cfg(all(test, feature = "coverage-controls"))]
#[allow(clippy::expect_used)]
mod coverage_tests {
    use super::*;

    fn mount_input(id: &str, prefix: &str) -> MountInput {
        MountInput {
            id: id.to_owned(),
            prefix: prefix.to_owned(),
        }
    }

    fn selector_input(mount: Option<&str>, root: &str) -> SelectorInput {
        SelectorInput {
            mount: mount.map(str::to_owned),
            root: root.to_owned(),
            adapter: String::from("markdown/v1"),
            include: vec![String::from("**/*.md")],
            exclude: vec![String::from("src/excluded/**")],
            authority_expectation: String::from("optional"),
        }
    }

    #[test]
    fn unit_variant_covers_mount_configuration_and_projection_failures() {
        assert_eq!(
            parse_mount_declarations(false, vec![mount_input("project", "project")]),
            Err(RepositoryError::ConfigSchema)
        );
        assert_eq!(
            parse_mount_declarations(true, Vec::new()),
            Err(RepositoryError::MountMissing)
        );
        assert_eq!(
            parse_mount_declarations(true, vec![mount_input("Bad", "project")]),
            Err(RepositoryError::MountId)
        );
        assert_eq!(
            parse_mount_declarations(true, vec![mount_input("project", "/invalid")]),
            Err(RepositoryError::MountPrefix)
        );
        for (error, expected) in [
            (
                LogicalMountError::DuplicateId,
                RepositoryError::MountIdDuplicate,
            ),
            (
                LogicalMountError::PrefixOverlap,
                RepositoryError::MountPrefixOverlap,
            ),
            (LogicalMountError::Id, RepositoryError::MountId),
            (LogicalMountError::Projection, RepositoryError::MountPrefix),
        ] {
            assert_eq!(project_mount_error(error), expected);
        }
        let mounts = canonical_logical_mounts(vec![LogicalMount::new(
            LogicalMountId::new("project").expect("mount ID"),
            RepositoryPath::new("project").expect("mount prefix"),
        )])
        .expect("mount set");
        assert_eq!(
            parse_selectors(true, vec![selector_input(None, "src")], &mounts),
            Err(RepositoryError::SelectorMount)
        );
        assert_eq!(
            parse_selectors(true, vec![selector_input(Some("Bad"), "src")], &mounts,),
            Err(RepositoryError::SelectorMount)
        );
        let mut invalid_exclude = selector_input(Some("project"), "src");
        invalid_exclude.exclude = vec![String::from("[")];
        assert_eq!(
            parse_selectors(true, vec![invalid_exclude], &mounts),
            Err(RepositoryError::GlobSyntax)
        );
        assert_eq!(
            parse_selectors(false, vec![selector_input(Some("project"), "src")], &[]),
            Err(RepositoryError::ConfigSchema)
        );
        assert_eq!(
            parse_selectors(
                true,
                vec![
                    selector_input(Some("project"), "src"),
                    selector_input(Some("project"), "src"),
                ],
                &mounts,
            ),
            Err(RepositoryError::ConfigSchema)
        );
        let selectors =
            parse_selectors(true, vec![selector_input(Some("project"), "src")], &mounts)
                .expect("selector set");
        let selector = &selectors[0];
        assert!(!selector.selects_path(&RepositoryPath::new("src").expect("root")));
        assert!(
            selector.selects_path(&RepositoryPath::new("src/control.md").expect("included path"))
        );
        assert!(
            !selector.selects_path(
                &RepositoryPath::new("src/excluded/control.md").expect("excluded path")
            )
        );
        assert!(
            !selector
                .selects_path(&RepositoryPath::new("outside/control.md").expect("outside path"))
        );

        assert_eq!(
            MountMappings::parse(&[String::from("project=")]),
            Err(RepositoryError::MountMapping)
        );
        assert_eq!(
            MountMappings::parse(&[String::from("Bad=path")]),
            Err(RepositoryError::MountMapping)
        );
        let id = LogicalMountId::new("project").expect("mount ID");
        assert_eq!(
            MountMappings::new(vec![
                (id.clone(), PathBuf::from("first")),
                (id, PathBuf::from("second")),
            ]),
            Err(RepositoryError::MountMappingDuplicate)
        );
    }

    #[cfg(unix)]
    #[test]
    fn unit_variant_covers_mounted_snapshot_wrapper_and_file_root_controls() {
        let decomposed = char::from_u32(0x301).expect("combining accent");
        let noncanonical = format!(
            "schema = \"spec-provenance-config/v2\"\ndigest_algorithms = [\"xxh3-128\"]\n\n[[selectors]]\nroot = \"cafe{decomposed}\"\nadapter = \"artifact/v1\"\ninclude = [\"**/*\"]\nexclude = []\nauthority_expectation = \"forbidden\"\n"
        );
        assert_eq!(
            RepositoryConfig::parse(noncanonical.as_bytes()),
            Err(RepositoryError::ConfigSchema)
        );
        let invalid_oracle_mount = br#"schema = "buoy/repository-config/v3"
digest_algorithms = ["xxh3-128"]

[[mounts]]
id = "project"
prefix = "project"

[[selectors]]
mount = "project"
root = "src"
adapter = "artifact/v1"
include = ["**/*"]
exclude = []
authority_expectation = "forbidden"

[[oracles]]
id = "ORC-CONTROL"
program = "builtin:pass"
working_root = "Bad"
timeout_ms = 1000
"#;
        assert_eq!(
            RepositoryConfig::parse(invalid_oracle_mount),
            Err(RepositoryError::OracleMount)
        );

        let git_root = tempfile::TempDir::new().expect("Git selector root");
        fs::create_dir(git_root.path().join(".git")).expect("Git directory");
        let git_config = RepositoryConfig::parse(
            br#"schema = "spec-provenance-config/v2"
digest_algorithms = ["xxh3-128"]

[[selectors]]
root = ".git"
adapter = "artifact/v1"
include = ["**/*"]
exclude = []
authority_expectation = "forbidden"
"#,
        )
        .expect("Git selector configuration");
        assert_eq!(
            snapshot_repository(git_root.path(), &git_config),
            Err(RepositoryError::SubmoduleEntry)
        );

        let mounted = RepositoryConfig::parse(include_bytes!(
            "../fixtures/logical-mount-artifact-bindings/v1/configs/one-mount.toml"
        ))
        .expect("mounted configuration");
        let root = tempfile::TempDir::new().expect("mounted root");
        assert_eq!(
            snapshot_repository(root.path(), &mounted),
            Err(RepositoryError::RootMountMixed)
        );
        assert_eq!(
            snapshot_repository_after_root_open_control(
                root.path(),
                &mounted,
                std::thread::yield_now,
            ),
            Err(RepositoryError::RootMountMixed)
        );
        assert_eq!(
            snapshot_repository_fault_control(root.path(), &mounted, "root-stat"),
            Err(RepositoryError::RootMountMixed)
        );

        fs::write(root.path().join("src"), b"artifact").expect("mounted file root");
        let declaration = &mounted.mounts()[0];
        let mappings =
            MountMappings::new(vec![(declaration.id().clone(), root.path().to_path_buf())])
                .expect("mounted mappings");
        let missing_mappings = MountMappings::new(Vec::new()).expect("missing mappings");
        assert_eq!(
            snapshot_mounted_repository_after_roots_open_control(
                &missing_mappings,
                &mounted,
                std::thread::yield_now,
            ),
            Err(RepositoryError::MountMissing)
        );
        assert_eq!(
            snapshot_mounted_repository_fault_control(&missing_mappings, &mounted, "root-stat"),
            Err(RepositoryError::MountMissing)
        );
        assert!(
            snapshot_mounted_repository_after_roots_open_control(
                &mappings,
                &mounted,
                std::thread::yield_now,
            )
            .is_ok()
        );
        let mut after_open = || {};
        assert_eq!(
            snapshot_repository_after_roots_open(
                &[(Some(declaration), root.path())],
                &mounted,
                &mut after_open,
                Some("opened-root-stat"),
            ),
            Err(RepositoryError::MountRoot)
        );
        assert_eq!(
            snapshot_repository_after_roots_open(
                &[(Some(declaration), root.path())],
                &mounted,
                &mut after_open,
                Some("observed-size"),
            ),
            Err(RepositoryError::FileBytesBudget)
        );

        fs::remove_file(root.path().join("src")).expect("remove file root");
        fs::create_dir(root.path().join("src")).expect("source directory");
        fs::write(root.path().join("src/control.md"), b"control").expect("selected file");
        assert_eq!(
            snapshot_repository_after_roots_open(
                &[(Some(declaration), root.path())],
                &mounted,
                &mut after_open,
                Some("selector-depth"),
            ),
            Err(RepositoryError::PathDepthBudget)
        );
        assert_eq!(
            snapshot_repository_after_roots_open(
                &[(Some(declaration), root.path())],
                &mounted,
                &mut after_open,
                Some("selector-stat"),
            ),
            Err(RepositoryError::ReadFailure)
        );
        assert_eq!(
            snapshot_repository_after_roots_open(
                &[(Some(declaration), root.path())],
                &mounted,
                &mut after_open,
                Some("selector-type"),
            ),
            Err(RepositoryError::LinkEntry)
        );
        assert_eq!(
            snapshot_repository_after_roots_open(
                &[(Some(declaration), root.path())],
                &mounted,
                &mut after_open,
                Some("directory-read"),
            ),
            Err(RepositoryError::ReadFailure)
        );
        assert_eq!(
            snapshot_repository_after_roots_open(
                &[(Some(declaration), root.path())],
                &mounted,
                &mut after_open,
                Some("selected-paths"),
            ),
            Err(RepositoryError::LogicalPathCollision)
        );
    }

    #[test]
    fn unit_variant_covers_snapshot_encoding_failures() {
        let markdown = SnapshotFile {
            path: RepositoryPath::new("spec/coverage.md").expect("coverage path"),
            bytes: b"coverage".to_vec(),
            adapter: String::from("markdown/v1"),
            authority_expectation: AuthorityExpectation::Optional,
            selector_root: String::from("spec"),
        };
        assert!(markdown.canonical_value().is_ok());
        assert_eq!(SnapshotBudgets::default().graph_references, 2_000_000);
        let source_errors = [
            RepositoryError::SourceNodesBudget,
            RepositoryError::SourceEdgesBudget,
            RepositoryError::SourceDepthBudget,
            RepositoryError::SourceVisitedBudget,
        ];
        for (index, expected) in source_errors.into_iter().enumerate() {
            for value in [0, u64::from(SOURCE_BUDGET_MAXIMUM) + 1] {
                let mut budgets = SnapshotBudgets::default();
                match index {
                    0 => budgets.source_nodes = value,
                    1 => budgets.source_edges = value,
                    2 => budgets.source_depth = value,
                    _ => budgets.source_visited = value,
                }
                assert_eq!(budgets.source_budgets(), Err(expected));
            }
        }
        let invalid_source_budget = br#"schema = "spec-provenance-config/v2"
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
graph_objects = 200000
graph_references = 2000000
diagnostics = 200000
source_nodes = 0
source_edges = 400000
source_depth = 64
source_visited = 200000
"#;
        assert_eq!(
            RepositoryConfig::parse(invalid_source_budget),
            Err(RepositoryError::SourceNodesBudget)
        );
        assert_eq!(
            RepositoryError::SourceNodesBudget.code(),
            "SRG-BUDGET-NODES"
        );
        assert_eq!(
            RepositoryError::SourceEdgesBudget.code(),
            "SRG-BUDGET-EDGES"
        );
        assert_eq!(
            RepositoryError::SourceDepthBudget.code(),
            "SRG-BUDGET-DEPTH"
        );
        assert_eq!(
            RepositoryError::SourceVisitedBudget.code(),
            "SRG-BUDGET-VISITED"
        );
        assert_eq!(
            canonical_value_set(vec![CanonicalValue::Record(vec![
                (1, CanonicalValue::False),
                (1, CanonicalValue::True),
            ])]),
            Err(RepositoryError::SnapshotEncoding)
        );

        let unsupported = SnapshotFile {
            adapter: String::from("unsupported/v1"),
            ..markdown
        };
        assert_eq!(
            RepositorySnapshot {
                files: vec![unsupported],
            }
            .semantic_digest(
                SemanticDigest::xxh3_128([1; 16]),
                SemanticDigest::xxh3_128([2; 16]),
                &[],
            ),
            Err(RepositoryError::UnsupportedAdapter)
        );
    }
}
