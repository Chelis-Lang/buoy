//! Repository-local quality-assurance registry validation and generation.
//!
//! This module intentionally remains in the imperative root crate. It reads
//! repository files and `OpenSpec` ownership records; none of these effects or
//! values enter `buoy-core`.

#![allow(
    clippy::expect_used,
    clippy::items_after_statements,
    clippy::needless_pass_by_value,
    clippy::struct_excessive_bools,
    clippy::struct_field_names,
    clippy::too_many_lines
)]

#[path = "../tools/digest_binding.rs"]
mod content_digest;
#[path = "../tools/execution_evidence.rs"]
mod execution_evidence;

use serde::Deserialize;
use std::collections::{BTreeMap, BTreeSet};
use std::env;
use std::fmt::{self, Write as _};
use std::fs;
use std::path::{Component, Path, PathBuf};

const LIVE_REGISTRY: &str = "assurance/v1/quality-assurance-controls.toml";
const FIXTURE_REGISTRY: &str = "fixtures/quality/v1/positive-all-states.toml";
const FIXTURE_CASES: &str = "fixtures/quality/v1/cases.tsv";
const FIXTURE_EXECUTIONS: &str = "assurance/v1/quality-fixture-executions.tsv";
const EXECUTION_TOOL_PATHS: [&str; 4] = [
    "src/quality.rs",
    "src/bin/buoy-quality.rs",
    "tools/execution_evidence.rs",
    "tools/digest_binding.rs",
];
const EXECUTION_CONFIGURATION_PATHS: [&str; 2] = [FIXTURE_CASES, FIXTURE_REGISTRY];
const SCHEMA: &str = "quality-assurance-controls/v1";
const DIGEST_POLICY: &str = "xxh3-128";
const FINAL_COMMAND: &str = "devenv test";

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct Registry {
    schema: String,
    mode: RegistryMode,
    digest_policy: String,
    final_command: String,
    gate_source: String,
    report_path: String,
    bindings_path: String,
    #[serde(default)]
    retired_ids: Vec<String>,
    synthetic_inventory: Option<Inventory>,
    controls: Vec<Control>,
    #[serde(default)]
    non_applicable: Vec<Disposition>,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq)]
#[serde(rename_all = "kebab-case")]
enum RegistryMode {
    Live,
    Synthetic,
}

#[derive(Clone, Debug, Default, Deserialize)]
#[serde(deny_unknown_fields)]
struct Inventory {
    #[serde(default)]
    tools: Vec<String>,
    #[serde(default)]
    tasks: Vec<String>,
    #[serde(default)]
    scripts: Vec<String>,
    #[serde(default)]
    obligations: Vec<String>,
}

#[derive(Debug, Deserialize)]
struct WorkflowPolicy {
    #[serde(default)]
    script_owner: Vec<WorkflowScriptOwner>,
    #[serde(default)]
    runtime_package: Vec<WorkflowRuntimePackage>,
}

#[derive(Debug, Deserialize)]
struct WorkflowScriptOwner {
    scripts: Vec<String>,
}

#[derive(Debug, Deserialize)]
struct WorkflowRuntimePackage {
    packages: Vec<String>,
}

#[derive(Debug, Deserialize)]
struct ResolvedTask {
    name: String,
    #[serde(default)]
    after: Vec<String>,
    #[serde(default)]
    before: Vec<String>,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct Disposition {
    kind: AccountKind,
    id: String,
    disposition: DispositionKind,
    rationale: String,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq)]
#[serde(rename_all = "kebab-case")]
enum DispositionKind {
    NotApplicable,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq)]
#[serde(rename_all = "kebab-case")]
enum AccountKind {
    Tool,
    Task,
    Script,
    Obligation,
}

impl AccountKind {
    const fn prefix(self) -> &'static str {
        match self {
            Self::Tool => "tool",
            Self::Task => "task",
            Self::Script => "script",
            Self::Obligation => "obligation",
        }
    }
}

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct Control {
    id: String,
    name: String,
    category: Category,
    scope: String,
    implementation_state: ImplementationState,
    enforcement_state: EnforcementState,
    evidence_state: EvidenceState,
    execution_verdict: ExecutionVerdict,
    assurance_class: AssuranceClass,
    lifecycle: Lifecycle,
    focused_command: Option<String>,
    gate_invocation: Option<String>,
    gate_omission_control: Option<String>,
    #[serde(default)]
    tool_bindings: Vec<String>,
    #[serde(default)]
    implementation_paths: Vec<String>,
    #[serde(default)]
    configuration_paths: Vec<String>,
    #[serde(default)]
    evidence_paths: Vec<String>,
    #[serde(default)]
    positive_controls: Vec<String>,
    #[serde(default)]
    negative_controls: Vec<String>,
    #[serde(default)]
    documentation_paths: Vec<String>,
    #[serde(default)]
    evidence_credits: Vec<String>,
    #[serde(default)]
    accounts: Vec<String>,
    trust_statement: String,
    #[serde(default)]
    assumptions: Vec<String>,
    #[serde(default)]
    exclusions: Vec<String>,
    #[serde(default)]
    non_claims: Vec<String>,
    owner: Owner,
    blocker: Option<String>,
    discharge_condition: Option<String>,
    retirement_rationale: Option<String>,
    replacement: Option<String>,
    #[serde(default)]
    terminal_retirement: bool,
    #[serde(default)]
    replaces: Vec<String>,
    result: Option<ResultAuthority>,
    finite_model: Option<FiniteModel>,
    formal_binding: Option<FormalBinding>,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd)]
#[serde(rename_all = "kebab-case")]
enum Category {
    AdversarialCampaign,
    CodeHygiene,
    DeterministicFixtures,
    FiniteModel,
    FormalVerification,
    OrdinaryVerification,
    PlatformRuntime,
    RegistryGovernance,
    Reproducibility,
    SampledProperties,
    SourceCoverage,
    StructuralEnforcement,
    SupplyChain,
}

impl Category {
    const fn as_str(self) -> &'static str {
        match self {
            Self::AdversarialCampaign => "adversarial-campaign",
            Self::CodeHygiene => "code-hygiene",
            Self::DeterministicFixtures => "deterministic-fixtures",
            Self::FiniteModel => "finite-model",
            Self::FormalVerification => "formal-verification",
            Self::OrdinaryVerification => "ordinary-verification",
            Self::PlatformRuntime => "platform-runtime",
            Self::RegistryGovernance => "registry-governance",
            Self::Reproducibility => "reproducibility",
            Self::SampledProperties => "sampled-properties",
            Self::SourceCoverage => "source-coverage",
            Self::StructuralEnforcement => "structural-enforcement",
            Self::SupplyChain => "supply-chain",
        }
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq)]
#[serde(rename_all = "kebab-case")]
enum ImplementationState {
    Planned,
    Available,
    Implemented,
    Blocked,
    Retired,
}

impl ImplementationState {
    const fn as_str(self) -> &'static str {
        match self {
            Self::Planned => "planned",
            Self::Available => "available",
            Self::Implemented => "implemented",
            Self::Blocked => "blocked",
            Self::Retired => "retired",
        }
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq)]
#[serde(rename_all = "kebab-case")]
enum EnforcementState {
    None,
    Advisory,
    Focused,
    Blocking,
}

impl EnforcementState {
    const fn as_str(self) -> &'static str {
        match self {
            Self::None => "none",
            Self::Advisory => "advisory",
            Self::Focused => "focused",
            Self::Blocking => "blocking",
        }
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq)]
#[serde(rename_all = "kebab-case")]
enum EvidenceState {
    Absent,
    Current,
    Stale,
}

impl EvidenceState {
    const fn as_str(self) -> &'static str {
        match self {
            Self::Absent => "absent",
            Self::Current => "current",
            Self::Stale => "stale",
        }
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq)]
#[serde(rename_all = "kebab-case")]
enum ExecutionVerdict {
    NotRun,
    Pass,
    Fail,
    Error,
    Timeout,
}

impl ExecutionVerdict {
    const fn as_str(self) -> &'static str {
        match self {
            Self::NotRun => "not-run",
            Self::Pass => "pass",
            Self::Fail => "fail",
            Self::Error => "error",
            Self::Timeout => "timeout",
        }
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq)]
#[serde(rename_all = "kebab-case")]
enum AssuranceClass {
    None,
    Linked,
    Tested,
    Sampled,
    BoundedExhaustive,
    FormallyProved,
    StructurallyEnforced,
    ReviewOnly,
}

impl AssuranceClass {
    const fn as_str(self) -> &'static str {
        match self {
            Self::None => "none",
            Self::Linked => "linked",
            Self::Tested => "tested",
            Self::Sampled => "sampled",
            Self::BoundedExhaustive => "bounded-exhaustive",
            Self::FormallyProved => "formally-proved",
            Self::StructurallyEnforced => "structurally-enforced",
            Self::ReviewOnly => "review-only",
        }
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq)]
#[serde(rename_all = "kebab-case")]
enum Lifecycle {
    Active,
    Replacement,
    Retired,
}

impl Lifecycle {
    const fn as_str(self) -> &'static str {
        match self {
            Self::Active => "active",
            Self::Replacement => "replacement",
            Self::Retired => "retired",
        }
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq)]
#[serde(rename_all = "kebab-case")]
enum OwnerKind {
    Baseline,
    Change,
}

impl OwnerKind {
    const fn as_str(self) -> &'static str {
        match self {
            Self::Baseline => "baseline",
            Self::Change => "change",
        }
    }
}

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct Owner {
    kind: OwnerKind,
    authority: String,
    #[serde(default)]
    related_changes: Vec<String>,
    #[serde(default)]
    additional_claimants: Vec<String>,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct ResultAuthority {
    path: String,
    source_paths: Vec<String>,
    tool_bindings: Vec<String>,
    tool_paths: Vec<String>,
    configuration_paths: Vec<String>,
    scope: String,
    source_digest: String,
    tool_digest: String,
    configuration_digest: String,
    scope_digest: String,
    result_digest: String,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct FiniteModel {
    id: String,
    path: String,
    bounds: String,
    transitions: String,
    invariants: Vec<String>,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct FormalBinding {
    implementation_representation: String,
    proof_source: String,
    verifier: String,
    solver: String,
    configuration: String,
    correspondence: String,
    negative_control: String,
    scope: String,
    assumptions: Vec<String>,
    exclusions: Vec<String>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct Diagnostic {
    code: &'static str,
    control: String,
    path: String,
    message: String,
}

impl Diagnostic {
    fn new(
        code: &'static str,
        control: impl Into<String>,
        path: impl Into<String>,
        message: impl Into<String>,
    ) -> Self {
        Self {
            code,
            control: control.into(),
            path: path.into(),
            message: message.into(),
        }
    }
}

/// Deterministic quality-registry failure containing stable diagnostics.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct QualityError {
    diagnostics: Vec<Diagnostic>,
}

impl QualityError {
    fn from_diagnostic(diagnostic: Diagnostic) -> Self {
        Self {
            diagnostics: vec![diagnostic],
        }
    }

    fn from_diagnostics(mut diagnostics: Vec<Diagnostic>) -> Self {
        diagnostics.sort_by(|left, right| {
            (&left.control, &left.path, left.code, &left.message).cmp(&(
                &right.control,
                &right.path,
                right.code,
                &right.message,
            ))
        });
        diagnostics.dedup();
        Self { diagnostics }
    }

    fn code_count(&self, code: &str) -> usize {
        self.diagnostics
            .iter()
            .filter(|diagnostic| diagnostic.code == code)
            .count()
    }
}

impl fmt::Display for QualityError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut rendered = String::new();
        for (index, diagnostic) in self.diagnostics.iter().enumerate() {
            if index > 0 {
                rendered.push('\n');
            }
            rendered.push_str(diagnostic.code);
            if !diagnostic.control.is_empty() {
                let _ = write!(rendered, " [{}]", diagnostic.control);
            }
            if !diagnostic.path.is_empty() {
                let _ = write!(rendered, " {}", diagnostic.path);
            }
            let _ = write!(rendered, ": {}", diagnostic.message);
        }
        formatter.write_str(&rendered)
    }
}

impl std::error::Error for QualityError {}

fn expected_control_failure<T: fmt::Debug>(result: Result<T, QualityError>) {
    drop(result.expect_err("quality reachability control must fail"));
}

/// Byte-identical generated binding and Markdown artifacts.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct GeneratedArtifacts {
    /// Deterministic content-binding TSV bytes represented as UTF-8.
    pub bindings: String,
    /// Deterministic maintainer-facing Markdown report bytes.
    pub report: String,
}

fn read_text(root: &Path, relative: &str, code: &'static str) -> Result<String, QualityError> {
    fs::read_to_string(root.join(relative)).map_err(|error| {
        QualityError::from_diagnostic(Diagnostic::new(
            code,
            "",
            relative,
            format!("cannot read repository input: {error}"),
        ))
    })
}

fn parse_registry_text(text: &str, path: &str) -> Result<Registry, QualityError> {
    toml::from_str::<Registry>(text).map_err(|_| {
        QualityError::from_diagnostic(Diagnostic::new(
            "QA-SCHEMA",
            "",
            path,
            "registry does not match the closed quality-assurance-controls/v1 schema",
        ))
    })
}

fn parse_registry(root: &Path, relative: &str) -> Result<Registry, QualityError> {
    let text = read_text(root, relative, "QA-REGISTRY-READ")?;
    parse_registry_text(&text, relative)
}

fn valid_control_id(id: &str) -> bool {
    id.len() == 7 && id.starts_with("QA-") && id.as_bytes()[3..].iter().all(u8::is_ascii_digit)
}

fn validate_relative_path(path: &str) -> Option<&'static str> {
    let candidate = Path::new(path);
    if candidate.is_absolute() {
        return Some("QA-PATH-ABSOLUTE");
    }
    if path.is_empty() || path.contains('\\') {
        return Some("QA-PATH-TRAVERSAL");
    }
    for component in candidate.components() {
        match component {
            Component::Normal(_) => {}
            Component::CurDir
            | Component::ParentDir
            | Component::RootDir
            | Component::Prefix(_) => {
                return Some("QA-PATH-TRAVERSAL");
            }
        }
    }
    None
}

fn push_path_diagnostic(diagnostics: &mut Vec<Diagnostic>, root: &Path, control: &str, path: &str) {
    if let Some(code) = validate_relative_path(path) {
        diagnostics.push(Diagnostic::new(
            code,
            control,
            path,
            "authority path must be normalized repository-relative UTF-8 without traversal",
        ));
    } else if !root.join(path).is_file() {
        diagnostics.push(Diagnostic::new(
            "QA-PATH-MISSING",
            control,
            path,
            "referenced authority path is not a repository file",
        ));
    }
}

fn all_path_groups(control: &Control) -> [(&'static str, &[String]); 6] {
    [
        ("implementation", &control.implementation_paths),
        ("configuration", &control.configuration_paths),
        ("evidence", &control.evidence_paths),
        ("positive-control", &control.positive_controls),
        ("negative-control", &control.negative_controls),
        ("documentation", &control.documentation_paths),
    ]
}

fn validate_paths(root: &Path, registry: &Registry, diagnostics: &mut Vec<Diagnostic>) {
    for path in [
        registry.gate_source.as_str(),
        registry.report_path.as_str(),
        registry.bindings_path.as_str(),
    ] {
        if let Some(code) = validate_relative_path(path) {
            diagnostics.push(Diagnostic::new(
                code,
                "",
                path,
                "registry path must be normalized repository-relative UTF-8",
            ));
        }
    }

    for control in &registry.controls {
        for (_, paths) in all_path_groups(control) {
            let mut seen = BTreeSet::new();
            for path in paths {
                if !seen.insert(path) {
                    diagnostics.push(Diagnostic::new(
                        "QA-BINDING-DUPLICATE",
                        &control.id,
                        path,
                        "control repeats the same binding in one authority class",
                    ));
                    continue;
                }
                push_path_diagnostic(diagnostics, root, &control.id, path);
            }
        }
        push_path_diagnostic(diagnostics, root, &control.id, &control.owner.authority);
        for optional in [
            control.blocker.as_deref(),
            control.gate_omission_control.as_deref(),
        ]
        .into_iter()
        .flatten()
        {
            push_path_diagnostic(diagnostics, root, &control.id, optional);
        }
        if let Some(result) = &control.result {
            push_path_diagnostic(diagnostics, root, &control.id, &result.path);
            for path in result
                .source_paths
                .iter()
                .chain(result.tool_paths.iter())
                .chain(result.configuration_paths.iter())
            {
                push_path_diagnostic(diagnostics, root, &control.id, path);
            }
        }
        if let Some(model) = &control.finite_model {
            push_path_diagnostic(diagnostics, root, &control.id, &model.path);
        }
        if let Some(formal) = &control.formal_binding {
            for path in [
                &formal.implementation_representation,
                &formal.proof_source,
                &formal.configuration,
                &formal.negative_control,
            ] {
                push_path_diagnostic(diagnostics, root, &control.id, path);
            }
        }
    }
}

fn runtime_tool_id(package: &str) -> Option<&str> {
    match package {
        "aeneas" => Some("aeneas"),
        "cargo-deny" => Some("cargo-deny"),
        "cargo-fuzz" => Some("cargo-fuzz"),
        "cargo-llvm-cov" => Some("cargo-llvm-cov"),
        "cargo-mutants" => Some("cargo-mutants"),
        "dylint" => Some("dylint"),
        "lean-aeneas" => Some("lean"),
        "verus" => Some("verus"),
        "z3-standalone" => Some("z3-standalone"),
        _ => None,
    }
}

fn parse_resolved_tasks(path: &Path) -> Result<Vec<ResolvedTask>, QualityError> {
    let text = fs::read_to_string(path).map_err(|error| {
        QualityError::from_diagnostic(Diagnostic::new(
            "QA-GATE-READ",
            "",
            "DEVENV_TASK_FILE",
            format!("cannot read resolved task data: {error}"),
        ))
    })?;
    serde_json::from_str(&text).map_err(|error| {
        QualityError::from_diagnostic(Diagnostic::new(
            "QA-GATE-SCHEMA",
            "",
            "DEVENV_TASK_FILE",
            format!("cannot parse resolved task data: {error}"),
        ))
    })
}

fn resolved_tasks(root: &Path) -> Result<Vec<ResolvedTask>, QualityError> {
    resolved_tasks_from(env::var_os("DEVENV_TASK_FILE").map(PathBuf::from), root)
}

fn resolved_tasks_from(
    path: Option<PathBuf>,
    root: &Path,
) -> Result<Vec<ResolvedTask>, QualityError> {
    match path {
        Some(path) => parse_resolved_tasks(&path),
        None => parse_resolved_tasks(&root.join("fixtures/devenv/v1/valid-tasks.json")),
    }
}

fn resolved_task_prerequisites(
    tasks: &[ResolvedTask],
) -> Result<BTreeMap<String, BTreeSet<String>>, QualityError> {
    let names = tasks
        .iter()
        .map(|task| task.name.as_str())
        .collect::<BTreeSet<_>>();
    let mut prerequisites = names
        .iter()
        .map(|name| ((*name).to_owned(), BTreeSet::new()))
        .collect::<BTreeMap<_, _>>();
    for task in tasks {
        for dependency in &task.after {
            let dependency = dependency
                .split_once('@')
                .map_or(dependency.as_str(), |(name, _)| name);
            if !names.contains(dependency) {
                return Err(QualityError::from_diagnostic(Diagnostic::new(
                    "QA-GATE-SCHEMA",
                    "",
                    &task.name,
                    format!("unknown task dependency: {dependency}"),
                )));
            }
            prerequisites
                .get_mut(&task.name)
                .expect("resolved task prerequisite entry exists")
                .insert(dependency.to_owned());
        }
        for dependent in &task.before {
            let dependent = dependent
                .split_once('@')
                .map_or(dependent.as_str(), |(name, _)| name);
            if !names.contains(dependent) {
                return Err(QualityError::from_diagnostic(Diagnostic::new(
                    "QA-GATE-SCHEMA",
                    "",
                    &task.name,
                    format!("unknown task dependent: {dependent}"),
                )));
            }
            prerequisites
                .get_mut(dependent)
                .expect("resolved task dependent entry exists")
                .insert(task.name.clone());
        }
    }
    Ok(prerequisites)
}

fn gate_task_name(invocation: &str) -> Option<String> {
    match invocation {
        "cargo fmt --all -- --check" => Some("buoy:format".to_owned()),
        "cargo clippy --workspace --all-targets --all-features -- -D warnings" => {
            Some("buoy:clippy".to_owned())
        }
        "cargo test --workspace --all-targets --all-features" => Some("buoy:test".to_owned()),
        "cargo doc --workspace --no-deps" => Some("buoy:docs".to_owned()),
        "openspec validate --all --strict --no-interactive" => Some("buoy:specs".to_owned()),
        value if value.starts_with("buoy-") => Some(value.replacen("buoy-", "buoy:", 1)),
        _ => None,
    }
}

fn task_reaches(
    prerequisites: &BTreeMap<String, BTreeSet<String>>,
    start: &str,
    target: &str,
) -> bool {
    let mut pending = BTreeSet::from([target.to_owned()]);
    let mut visited = BTreeSet::new();
    while let Some(name) = pending.pop_first() {
        if name == start {
            return true;
        }
        if visited.insert(name.clone()) {
            if let Some(dependencies) = prerequisites.get(&name) {
                pending.extend(dependencies.iter().cloned());
            }
        }
    }
    false
}

fn discover_inventory(root: &Path, registry: &Registry) -> Result<Inventory, QualityError> {
    discover_inventory_from(
        root,
        registry,
        env::var_os("DEVENV_TASK_FILE").map(PathBuf::from),
    )
}

fn discover_inventory_from(
    root: &Path,
    registry: &Registry,
    task_path: Option<PathBuf>,
) -> Result<Inventory, QualityError> {
    if registry.mode == RegistryMode::Synthetic {
        return registry.synthetic_inventory.clone().ok_or_else(|| {
            QualityError::from_diagnostic(Diagnostic::new(
                "QA-SCHEMA",
                "",
                FIXTURE_REGISTRY,
                "synthetic registry requires synthetic_inventory",
            ))
        });
    }
    if registry.synthetic_inventory.is_some() {
        return Err(QualityError::from_diagnostic(Diagnostic::new(
            "QA-SCHEMA",
            "",
            LIVE_REGISTRY,
            "live registry must discover inventory from repository authority",
        )));
    }

    #[derive(Deserialize)]
    struct Toolchain {
        #[serde(default)]
        tool: Vec<Tool>,
    }
    #[derive(Deserialize)]
    struct Tool {
        id: String,
        role: String,
    }

    let toolchain_text = read_text(root, "assurance/v1/toolchain.toml", "QA-DISCOVERY-READ")?;
    let toolchain = toml::from_str::<Toolchain>(&toolchain_text).map_err(|_| {
        QualityError::from_diagnostic(Diagnostic::new(
            "QA-DISCOVERY-SCHEMA",
            "",
            "assurance/v1/toolchain.toml",
            "cannot parse QA tool inventory",
        ))
    })?;
    let mut tools = toolchain
        .tool
        .into_iter()
        .filter(|tool| !tool.role.trim().is_empty())
        .map(|tool| tool.id)
        .collect::<BTreeSet<_>>();

    tools.insert("cargo-nextest".to_owned());
    tools.insert("openspec".to_owned());
    let policy_text = read_text(root, "nix/devenv/workflow-policy.toml", "QA-DISCOVERY-READ")?;
    let policy = toml::from_str::<WorkflowPolicy>(&policy_text).map_err(|_| {
        QualityError::from_diagnostic(Diagnostic::new(
            "QA-DISCOVERY-SCHEMA",
            "",
            "nix/devenv/workflow-policy.toml",
            "cannot parse the Devenv workflow policy",
        ))
    })?;
    for package in policy
        .runtime_package
        .iter()
        .flat_map(|runtime| &runtime.packages)
    {
        if let Some(tool) = runtime_tool_id(package) {
            tools.insert(tool.to_owned());
        }
    }
    let tasks = resolved_tasks_from(task_path, root)?
        .into_iter()
        .filter(|task| task.name.starts_with("buoy:"))
        .map(|task| task.name)
        .collect::<BTreeSet<_>>();
    let scripts = policy
        .script_owner
        .into_iter()
        .flat_map(|owner| owner.scripts)
        .collect::<BTreeSet<_>>();

    let handoffs_text = read_text(
        root,
        "assurance/v1/phase-handoffs.toml",
        "QA-DISCOVERY-READ",
    )?;
    let handoffs = toml::from_str::<toml::Value>(&handoffs_text).map_err(|_| {
        QualityError::from_diagnostic(Diagnostic::new(
            "QA-DISCOVERY-SCHEMA",
            "",
            "assurance/v1/phase-handoffs.toml",
            "cannot parse phase-handoff obligations",
        ))
    })?;
    let mut obligations = BTreeSet::new();
    if let Some(phases) = handoffs.get("phase").and_then(toml::Value::as_array) {
        for phase in phases {
            let Some(id) = phase.get("id").and_then(toml::Value::as_str) else {
                continue;
            };
            if let Some(requires) = phase.get("requires").and_then(toml::Value::as_array) {
                for requirement in requires.iter().filter_map(toml::Value::as_str) {
                    obligations.insert(format!("{id}:{requirement}"));
                }
            }
        }
    }

    Ok(Inventory {
        tools: tools.into_iter().collect(),
        tasks: tasks.into_iter().collect(),
        scripts: scripts.into_iter().collect(),
        obligations: obligations.into_iter().collect(),
    })
}

fn account_set(registry: &Registry) -> BTreeSet<String> {
    let controls = registry
        .controls
        .iter()
        .flat_map(|control| control.accounts.iter().cloned());
    let dispositions = registry
        .non_applicable
        .iter()
        .map(|disposition| format!("{}:{}", disposition.kind.prefix(), disposition.id));
    controls.chain(dispositions).collect()
}

fn validate_dispositions(registry: &Registry, diagnostics: &mut Vec<Diagnostic>) {
    let mut seen = BTreeSet::new();
    for disposition in &registry.non_applicable {
        let _ = disposition.disposition;
        let key = format!("{}:{}", disposition.kind.prefix(), disposition.id);
        if !seen.insert(key.clone()) {
            diagnostics.push(Diagnostic::new(
                "QA-DISPOSITION-DUPLICATE",
                "",
                &key,
                "non-applicable identity is repeated",
            ));
        }
        if disposition.rationale.trim().is_empty() {
            diagnostics.push(Diagnostic::new(
                "QA-DISPOSITION",
                "",
                key,
                "non-applicable disposition requires an exact rationale",
            ));
        }
    }
}

fn validate_completeness(
    inventory: &Inventory,
    registry: &Registry,
    diagnostics: &mut Vec<Diagnostic>,
) {
    let accounted = account_set(registry);
    for (kind, code, values) in [
        ("tool", "QA-COMPLETE-TOOL", &inventory.tools),
        ("task", "QA-COMPLETE-TASK", &inventory.tasks),
        ("script", "QA-COMPLETE-SCRIPT", &inventory.scripts),
        (
            "obligation",
            "QA-COMPLETE-OBLIGATION",
            &inventory.obligations,
        ),
    ] {
        for value in values {
            let key = format!("{kind}:{value}");
            if !accounted.contains(&key) {
                diagnostics.push(Diagnostic::new(
                    code,
                    "",
                    &key,
                    "discovered QA identity has no control or explicit non-applicable disposition",
                ));
            }
        }
    }
}

fn validate_identity(registry: &Registry, diagnostics: &mut Vec<Diagnostic>) {
    let retired = registry.retired_ids.iter().collect::<BTreeSet<_>>();
    let mut ids = BTreeSet::new();
    for control in &registry.controls {
        if !valid_control_id(&control.id) {
            diagnostics.push(Diagnostic::new(
                "QA-ID",
                &control.id,
                "",
                "control identity must match QA- followed by exactly four digits",
            ));
        }
        if !ids.insert(&control.id) {
            diagnostics.push(Diagnostic::new(
                "QA-ID-DUPLICATE",
                &control.id,
                "",
                "control identity is duplicated",
            ));
        }
        if retired.contains(&control.id) {
            diagnostics.push(Diagnostic::new(
                "QA-ID-RETIRED",
                &control.id,
                "",
                "retired control identity must never be reused",
            ));
        }
    }
}

fn task_from_command(command: &str) -> Option<&str> {
    command.strip_prefix("devenv tasks run ")
}

fn validate_states(inventory: &Inventory, registry: &Registry, diagnostics: &mut Vec<Diagnostic>) {
    let tasks = inventory
        .tasks
        .iter()
        .map(String::as_str)
        .collect::<BTreeSet<_>>();
    let tools = inventory
        .tools
        .iter()
        .map(String::as_str)
        .collect::<BTreeSet<_>>();
    for control in &registry.controls {
        for tool in &control.tool_bindings {
            if !tools.contains(tool.as_str()) {
                diagnostics.push(Diagnostic::new(
                    "QA-TOOL-BINDING",
                    &control.id,
                    tool,
                    "control references a tool absent from the authoritative QA inventory",
                ));
            }
        }
        match control.implementation_state {
            ImplementationState::Planned => {}
            ImplementationState::Available => {
                if control.tool_bindings.is_empty() && control.configuration_paths.is_empty() {
                    diagnostics.push(Diagnostic::new(
                        "QA-STATE-AVAILABLE",
                        &control.id,
                        "",
                        "available control requires an exact tool or configuration identity",
                    ));
                }
            }
            ImplementationState::Implemented => {
                if control.implementation_paths.is_empty() {
                    diagnostics.push(Diagnostic::new(
                        "QA-STATE-IMPLEMENTED",
                        &control.id,
                        "",
                        "implemented control requires executable project behavior, not only a pin",
                    ));
                } else {
                    if control.positive_controls.is_empty() {
                        diagnostics.push(Diagnostic::new(
                            "QA-STATE-POSITIVE",
                            &control.id,
                            "",
                            "implemented control requires an applicable positive control",
                        ));
                    }
                    if control.negative_controls.is_empty() {
                        diagnostics.push(Diagnostic::new(
                            "QA-STATE-NEGATIVE",
                            &control.id,
                            "",
                            "implemented control requires an applicable negative control",
                        ));
                    }
                }
            }
            ImplementationState::Blocked => {
                if control.blocker.as_deref().is_none_or(str::is_empty)
                    || control
                        .discharge_condition
                        .as_deref()
                        .is_none_or(str::is_empty)
                {
                    diagnostics.push(Diagnostic::new(
                        "QA-STATE-BLOCKED",
                        &control.id,
                        "",
                        "blocked control requires a repository blocker and discharge condition",
                    ));
                }
            }
            ImplementationState::Retired => {
                if control
                    .retirement_rationale
                    .as_deref()
                    .is_none_or(str::is_empty)
                    || (control.replacement.is_none() && !control.terminal_retirement)
                {
                    diagnostics.push(Diagnostic::new(
                        "QA-STATE-RETIRED",
                        &control.id,
                        "",
                        "retired control requires terminal rationale and replacement or terminal disposition",
                    ));
                }
            }
        }

        if control.enforcement_state == EnforcementState::Focused {
            let resolved = control
                .focused_command
                .as_deref()
                .and_then(task_from_command)
                .is_some_and(|task| tasks.contains(task));
            if !resolved {
                diagnostics.push(Diagnostic::new(
                    "QA-ENFORCEMENT-FOCUSED",
                    &control.id,
                    "",
                    "focused enforcement requires an exact discovered task command",
                ));
            }
        }
        if control.enforcement_state == EnforcementState::Blocking {
            let resolved = control
                .focused_command
                .as_deref()
                .and_then(task_from_command)
                .is_some_and(|task| tasks.contains(task));
            if !resolved
                || control.gate_invocation.as_deref().is_none_or(str::is_empty)
                || control
                    .gate_omission_control
                    .as_deref()
                    .is_none_or(str::is_empty)
            {
                diagnostics.push(Diagnostic::new(
                    "QA-ENFORCEMENT-BLOCKING",
                    &control.id,
                    "",
                    "blocking enforcement requires focused registration, exact gate invocation, and omission control",
                ));
            }
        }
    }
}

fn validate_gate(root: &Path, registry: &Registry, diagnostics: &mut Vec<Diagnostic>) {
    let source = if registry.mode == RegistryMode::Live {
        "DEVENV_TASK_FILE"
    } else {
        &registry.gate_source
    };
    let tasks = match if registry.mode == RegistryMode::Live {
        resolved_tasks(root)
    } else {
        parse_resolved_tasks(&root.join(&registry.gate_source))
    } {
        Ok(tasks) => tasks,
        Err(error) => {
            diagnostics.push(Diagnostic::new(
                "QA-GATE-READ",
                "",
                source,
                error.to_string(),
            ));
            return;
        }
    };
    let prerequisites = match resolved_task_prerequisites(&tasks) {
        Ok(prerequisites) => prerequisites,
        Err(error) => {
            diagnostics.push(Diagnostic::new(
                "QA-GATE-SCHEMA",
                "",
                source,
                error.to_string(),
            ));
            return;
        }
    };
    for control in registry
        .controls
        .iter()
        .filter(|control| control.enforcement_state == EnforcementState::Blocking)
    {
        if let Some(invocation) = &control.gate_invocation {
            let resolved = gate_task_name(invocation)
                .is_some_and(|task| task_reaches(&prerequisites, &task, "devenv:enterTest"));
            if !resolved {
                diagnostics.push(Diagnostic::new(
                    "QA-GATE-OMITTED",
                    &control.id,
                    source,
                    "blocking control has no resolved path to devenv:enterTest",
                ));
            }
        }
    }
}

fn digest_bytes(bytes: &[u8]) -> String {
    content_digest::bytes_binding(bytes)
}

fn digest_paths(root: &Path, paths: &[String]) -> Result<String, QualityError> {
    let mut sorted = paths.to_vec();
    sorted.sort();
    let mut bytes = Vec::new();
    for path in sorted {
        let content = fs::read(root.join(&path)).map_err(|error| {
            QualityError::from_diagnostic(Diagnostic::new(
                "QA-PATH-MISSING",
                "",
                &path,
                format!("cannot digest authority path: {error}"),
            ))
        })?;
        bytes.extend_from_slice(path.as_bytes());
        bytes.push(0);
        bytes.extend_from_slice(&content);
        bytes.push(0);
    }
    Ok(digest_bytes(&bytes))
}

fn file_digest(root: &Path, path: &str) -> Result<String, QualityError> {
    fs::read(root.join(path))
        .map(|bytes| digest_bytes(&bytes))
        .map_err(|error| {
            QualityError::from_diagnostic(Diagnostic::new(
                "QA-PATH-MISSING",
                "",
                path,
                format!("cannot digest authority path: {error}"),
            ))
        })
}

fn digest_tool_authority(root: &Path, result: &ResultAuthority) -> Result<String, QualityError> {
    let mut bytes = Vec::new();
    let mut tools = result.tool_bindings.clone();
    tools.sort();
    for tool in tools {
        bytes.extend_from_slice(tool.as_bytes());
        bytes.push(0);
    }
    let mut paths = result.tool_paths.clone();
    paths.sort();
    for path in paths {
        let content = fs::read(root.join(&path)).map_err(|error| {
            QualityError::from_diagnostic(Diagnostic::new(
                "QA-PATH-MISSING",
                "",
                &path,
                format!("cannot digest tool authority path: {error}"),
            ))
        })?;
        bytes.extend_from_slice(path.as_bytes());
        bytes.push(0);
        bytes.extend_from_slice(&content);
        bytes.push(0);
    }
    Ok(digest_bytes(&bytes))
}

fn result_record_matches(root: &Path, result: &ResultAuthority, verdict: ExecutionVerdict) -> bool {
    let Ok(text) = fs::read_to_string(root.join(&result.path)) else {
        return false;
    };
    let mut lines = text.lines();
    if lines.next() != Some("schema\tverdict\tscope") {
        return false;
    }
    let Some(row) = lines.next() else {
        return false;
    };
    if lines.next().is_some() {
        return false;
    }
    let fields = row.split('\t').collect::<Vec<_>>();
    fields.len() == 3
        && fields[0] == "quality-assurance-result/v1"
        && fields[1] == verdict.as_str()
        && fields[2] == result.scope
}

fn result_is_current(root: &Path, result: &ResultAuthority) -> bool {
    let source = digest_paths(root, &result.source_paths);
    let tool = digest_tool_authority(root, result);
    let configuration = digest_paths(root, &result.configuration_paths);
    let file = file_digest(root, &result.path);
    source.is_ok_and(|digest| digest == result.source_digest)
        && tool.is_ok_and(|digest| digest == result.tool_digest)
        && configuration.is_ok_and(|digest| digest == result.configuration_digest)
        && digest_bytes(result.scope.as_bytes()) == result.scope_digest
        && file.is_ok_and(|digest| digest == result.result_digest)
        && !result.scope.trim().is_empty()
}

fn validate_results(root: &Path, registry: &Registry, diagnostics: &mut Vec<Diagnostic>) {
    for control in &registry.controls {
        if control.execution_verdict == ExecutionVerdict::NotRun {
            if control.result.is_some() {
                diagnostics.push(Diagnostic::new(
                    "QA-RESULT-AUTHORITY",
                    &control.id,
                    "",
                    "not-run control must not bind a result authority",
                ));
            }
            continue;
        }
        let Some(result) = &control.result else {
            diagnostics.push(Diagnostic::new(
                "QA-RESULT-AUTHORITY",
                &control.id,
                "",
                "execution verdict requires normalized source/tool/configuration/scope/result authority",
            ));
            continue;
        };
        if !result_record_matches(root, result, control.execution_verdict) {
            diagnostics.push(Diagnostic::new(
                "QA-RESULT-MISMATCH",
                &control.id,
                &result.path,
                "normalized result authority does not supply the declared exact verdict and scope",
            ));
            continue;
        }
        if control.evidence_state != EvidenceState::Current || !result_is_current(root, result) {
            diagnostics.push(Diagnostic::new(
                "QA-RESULT-STALE",
                &control.id,
                &result.path,
                "execution result is stale for current source, tool, configuration, scope, or result bytes",
            ));
        }
    }
}

fn validate_ownership(registry: &Registry, diagnostics: &mut Vec<Diagnostic>) {
    let mut credits = BTreeMap::<&str, &str>::new();
    for control in &registry.controls {
        if control.owner.authority.trim().is_empty()
            || !control.owner.additional_claimants.is_empty()
            || control
                .owner
                .related_changes
                .iter()
                .any(|related| related == &control.owner.authority)
        {
            diagnostics.push(Diagnostic::new(
                "QA-OWNER-AMBIGUOUS",
                &control.id,
                &control.owner.authority,
                "control must have exactly one implementation owner; related changes are references only",
            ));
        }
        if control.owner.kind == OwnerKind::Baseline
            && control.owner.authority.starts_with("openspec/changes/")
            && !control
                .owner
                .authority
                .starts_with("openspec/changes/archive/")
        {
            diagnostics.push(Diagnostic::new(
                "QA-OWNER-AUTHORITY",
                &control.id,
                &control.owner.authority,
                "active OpenSpec ownership must be classified as change, not completed baseline",
            ));
        }
        for credit in &control.evidence_credits {
            if let Some(previous) = credits.insert(credit, &control.id) {
                if previous != control.id {
                    diagnostics.push(Diagnostic::new(
                        "QA-EVIDENCE-DUPLICATE",
                        &control.id,
                        credit,
                        format!("evidence credit is already assigned to {previous}"),
                    ));
                }
            }
        }
        if control.implementation_state == ImplementationState::Implemented
            && !control.implementation_paths.is_empty()
            && control
                .implementation_paths
                .iter()
                .all(|path| path.starts_with("openspec/changes/") && path.ends_with("/tasks.md"))
        {
            diagnostics.push(Diagnostic::new(
                "QA-EVIDENCE-CHECKBOX",
                &control.id,
                "",
                "OpenSpec task checkboxes are not technical implementation evidence",
            ));
        }
    }
}

fn validate_lifecycle(registry: &Registry, diagnostics: &mut Vec<Diagnostic>) {
    let controls = registry
        .controls
        .iter()
        .map(|control| (control.id.as_str(), control))
        .collect::<BTreeMap<_, _>>();
    for control in &registry.controls {
        if let Some(replacement) = &control.replacement {
            let complete = controls
                .get(replacement.as_str())
                .is_some_and(|target| target.replaces.contains(&control.id));
            if !complete {
                diagnostics.push(Diagnostic::new(
                    "QA-LIFECYCLE-REPLACEMENT",
                    &control.id,
                    replacement,
                    "replacement target must exist and carry the retired identity backlink",
                ));
            }
        }
        if control.lifecycle == Lifecycle::Retired
            && control.implementation_state != ImplementationState::Retired
        {
            diagnostics.push(Diagnostic::new(
                "QA-LIFECYCLE",
                &control.id,
                "",
                "retired lifecycle requires retired implementation state",
            ));
        }
    }
}

fn complete_formal_binding(binding: &FormalBinding) -> bool {
    [
        binding.implementation_representation.as_str(),
        binding.proof_source.as_str(),
        binding.verifier.as_str(),
        binding.solver.as_str(),
        binding.configuration.as_str(),
        binding.correspondence.as_str(),
        binding.negative_control.as_str(),
        binding.scope.as_str(),
    ]
    .into_iter()
    .all(|value| !value.trim().is_empty())
        && !binding.assumptions.is_empty()
        && !binding.exclusions.is_empty()
}

fn validate_assurance(registry: &Registry, diagnostics: &mut Vec<Diagnostic>) {
    for control in &registry.controls {
        if control.category == Category::SampledProperties
            && control.assurance_class != AssuranceClass::Sampled
        {
            diagnostics.push(Diagnostic::new(
                "QA-ASSURANCE-SAMPLED",
                &control.id,
                "",
                "sampled property evidence may not be promoted to exhaustive or formal assurance",
            ));
            continue;
        }
        if control.category == Category::SourceCoverage
            && control.assurance_class == AssuranceClass::FormallyProved
        {
            diagnostics.push(Diagnostic::new(
                "QA-ASSURANCE-COVERAGE",
                &control.id,
                "",
                "source coverage is structural execution evidence and may not be promoted to proof",
            ));
            continue;
        }
        if control.assurance_class == AssuranceClass::BoundedExhaustive {
            let complete = control.finite_model.as_ref().is_some_and(|model| {
                !model.id.trim().is_empty()
                    && !model.path.trim().is_empty()
                    && !model.bounds.trim().is_empty()
                    && !model.transitions.trim().is_empty()
                    && !model.invariants.is_empty()
            });
            if !complete {
                diagnostics.push(Diagnostic::new(
                    "QA-ASSURANCE-MODEL",
                    &control.id,
                    "",
                    "bounded-exhaustive claim requires exact model, bounds, transitions, and invariants",
                ));
            }
        }
        if control.assurance_class == AssuranceClass::FormallyProved
            && !control
                .formal_binding
                .as_ref()
                .is_some_and(complete_formal_binding)
        {
            diagnostics.push(Diagnostic::new(
                "QA-ASSURANCE-FORMAL",
                &control.id,
                "",
                "formal claim requires complete representation, proof, verifier, solver, configuration, correspondence, assumptions, negative control, scope, and exclusions",
            ));
        }
        if control.trust_statement.trim().is_empty()
            || control.exclusions.is_empty()
            || control.non_claims.is_empty()
        {
            diagnostics.push(Diagnostic::new(
                "QA-ASSURANCE-SCOPE",
                &control.id,
                "",
                "every control requires trust statement, exclusions, and non-claims",
            ));
        }
    }
}

fn validate_evidence_state(registry: &Registry, diagnostics: &mut Vec<Diagnostic>) {
    for control in &registry.controls {
        if control.evidence_state == EvidenceState::Current
            && control.evidence_paths.is_empty()
            && control.implementation_state != ImplementationState::Planned
        {
            diagnostics.push(Diagnostic::new(
                "QA-EVIDENCE-CURRENT",
                &control.id,
                "",
                "current evidence state requires explicit evidence authority",
            ));
        }
    }
}

fn validate_registry(root: &Path, registry: &Registry) -> Result<Inventory, QualityError> {
    let mut diagnostics = Vec::new();
    if registry.schema != SCHEMA {
        diagnostics.push(Diagnostic::new(
            "QA-SCHEMA-ID",
            "",
            LIVE_REGISTRY,
            "registry schema identity must be quality-assurance-controls/v1",
        ));
    }
    if registry.digest_policy != DIGEST_POLICY {
        diagnostics.push(Diagnostic::new(
            "QA-DIGEST-POLICY",
            "",
            LIVE_REGISTRY,
            "content bindings must use the active repository XXH3-128 policy",
        ));
    }
    if registry.final_command != FINAL_COMMAND {
        diagnostics.push(Diagnostic::new(
            "QA-FINAL-COMMAND",
            "",
            LIVE_REGISTRY,
            "the only final implementation acceptance oracle is devenv test",
        ));
    }

    let inventory = discover_inventory(root, registry)?;
    validate_identity(registry, &mut diagnostics);
    validate_paths(root, registry, &mut diagnostics);
    validate_dispositions(registry, &mut diagnostics);
    validate_completeness(&inventory, registry, &mut diagnostics);
    validate_states(&inventory, registry, &mut diagnostics);
    validate_gate(root, registry, &mut diagnostics);
    validate_results(root, registry, &mut diagnostics);
    validate_ownership(registry, &mut diagnostics);
    validate_lifecycle(registry, &mut diagnostics);
    validate_assurance(registry, &mut diagnostics);
    validate_evidence_state(registry, &mut diagnostics);

    if diagnostics.is_empty() {
        Ok(inventory)
    } else {
        Err(QualityError::from_diagnostics(diagnostics))
    }
}

fn binding_paths(control: &Control) -> BTreeMap<String, BTreeSet<&'static str>> {
    let mut paths = BTreeMap::<String, BTreeSet<&'static str>>::new();
    for (kind, values) in all_path_groups(control) {
        for value in values {
            paths.entry(value.clone()).or_default().insert(kind);
        }
    }
    paths
        .entry(control.owner.authority.clone())
        .or_default()
        .insert("owner");
    for (kind, optional) in [
        ("blocker", control.blocker.as_ref()),
        ("gate-omission", control.gate_omission_control.as_ref()),
    ] {
        if let Some(value) = optional {
            paths.entry(value.clone()).or_default().insert(kind);
        }
    }
    if let Some(result) = &control.result {
        paths
            .entry(result.path.clone())
            .or_default()
            .insert("result");
        for value in &result.source_paths {
            paths
                .entry(value.clone())
                .or_default()
                .insert("result-source");
        }
        for value in &result.tool_paths {
            paths
                .entry(value.clone())
                .or_default()
                .insert("result-tool");
        }
        for value in &result.configuration_paths {
            paths
                .entry(value.clone())
                .or_default()
                .insert("result-configuration");
        }
    }
    if let Some(model) = &control.finite_model {
        paths
            .entry(model.path.clone())
            .or_default()
            .insert("finite-model");
    }
    if let Some(formal) = &control.formal_binding {
        for (kind, value) in [
            (
                "formal-representation",
                &formal.implementation_representation,
            ),
            ("formal-proof", &formal.proof_source),
            ("formal-configuration", &formal.configuration),
            ("formal-negative", &formal.negative_control),
        ] {
            paths.entry(value.clone()).or_default().insert(kind);
        }
    }
    paths
}

fn render_bindings(root: &Path, registry: &Registry) -> Result<String, QualityError> {
    let mut controls = registry.controls.iter().collect::<Vec<_>>();
    controls.sort_by(|left, right| left.id.as_bytes().cmp(right.id.as_bytes()));
    let mut output = String::from("schema\tcontrol_id\tkinds\tpath\tdigest\n");
    for control in controls {
        for (path, kinds) in binding_paths(control) {
            let digest = file_digest(root, &path)?;
            let kind = kinds.into_iter().collect::<Vec<_>>().join(",");
            let _ = writeln!(
                output,
                "quality-assurance-binding/v1\t{}\t{}\t{}\t{}",
                control.id, kind, path, digest
            );
        }
    }
    Ok(output)
}

fn markdown_cell(value: &str) -> String {
    value.replace('|', "\\|").replace(['\r', '\n'], "<br>")
}

fn render_report(registry: &Registry) -> String {
    let mut controls = registry.controls.iter().collect::<Vec<_>>();
    controls.sort_by(|left, right| {
        (left.category, left.id.as_bytes()).cmp(&(right.category, right.id.as_bytes()))
    });
    let mut output = String::from(
        "# Quality-assurance control status\n\n\
         Generated from `assurance/v1/quality-assurance-controls.toml`; do not edit status cells manually. \
         Registry validation checks registration and evidence bindings but does not execute inventoried child lanes.\n\n\
         | Category | ID | Control | Implementation | Enforcement | Focused command | Evidence | Verdict | Owner | Assurance | Scope and trust | Limitations |\n\
         |---|---|---|---|---|---|---|---|---|---|---|---|\n",
    );
    for control in controls {
        let command = control.focused_command.as_deref().unwrap_or("—");
        let owner = format!(
            "{}: `{}`",
            control.owner.kind.as_str(),
            control.owner.authority
        );
        let trust = if control.assumptions.is_empty() {
            control.trust_statement.clone()
        } else {
            format!(
                "{} Assumptions: {}",
                control.trust_statement,
                control.assumptions.join("; ")
            )
        };
        let limitations = control
            .exclusions
            .iter()
            .chain(control.non_claims.iter())
            .map(String::as_str)
            .collect::<Vec<_>>()
            .join("; ");
        let _ = writeln!(
            output,
            "| {} | `{}` | {} | {} / {} | {} | `{}` | {} | {} | {} | {} | {} — {} | {} |",
            control.category.as_str(),
            control.id,
            markdown_cell(&control.name),
            control.implementation_state.as_str(),
            control.lifecycle.as_str(),
            control.enforcement_state.as_str(),
            markdown_cell(command),
            control.evidence_state.as_str(),
            control.execution_verdict.as_str(),
            markdown_cell(&owner),
            control.assurance_class.as_str(),
            markdown_cell(&control.scope),
            markdown_cell(&trust),
            markdown_cell(&limitations),
        );
    }
    output.push_str(
        "\n## Boundary\n\nA `pass` is shown only when an applicable normalized result authority is current for exact source, tool, configuration, scope, and result bytes. Tool availability, registry membership, task checkboxes, static gate registration, historical results, and passing sibling lanes never synthesize implementation, execution, exhaustiveness, or proof.\n",
    );
    output
}

fn render_validated_registry(
    root: &Path,
    registry: &Registry,
) -> Result<GeneratedArtifacts, QualityError> {
    Ok(GeneratedArtifacts {
        bindings: render_bindings(root, registry)?,
        report: render_report(registry),
    })
}

fn generate_from_registry(
    root: &Path,
    registry: &Registry,
) -> Result<GeneratedArtifacts, QualityError> {
    validate_registry(root, registry)?;
    render_validated_registry(root, registry)
}

/// Validate the live registry and render deterministic artifacts without writing them.
///
/// # Errors
///
/// Returns stable diagnostics when an input cannot be read or any registry claim fails.
pub fn generated_artifacts(root: &Path) -> Result<GeneratedArtifacts, QualityError> {
    let registry = parse_registry(root, LIVE_REGISTRY)?;
    generate_from_registry(root, &registry)
}

fn compare_artifact(
    root: &Path,
    path: &str,
    expected: &str,
    code: &'static str,
) -> Result<(), QualityError> {
    let actual = read_text(root, path, code)?;
    if actual == expected {
        Ok(())
    } else {
        Err(QualityError::from_diagnostic(Diagnostic::new(
            code,
            "",
            path,
            "committed generated artifact differs from deterministic regeneration",
        )))
    }
}

/// Validate the live registry and require committed bindings and report to be byte-current.
///
/// # Errors
///
/// Returns stable diagnostics for invalid claims, stale bindings, or report drift.
pub fn check_repository(root: &Path) -> Result<(), QualityError> {
    let registry = parse_registry(root, LIVE_REGISTRY)?;
    let generated = generate_from_registry(root, &registry)?;
    compare_artifact(
        root,
        &registry.bindings_path,
        &generated.bindings,
        "QA-BINDINGS-DRIFT",
    )?;
    compare_artifact(
        root,
        &registry.report_path,
        &generated.report,
        "QA-REPORT-DRIFT",
    )
}

fn write_generated(
    root: &Path,
    output: Option<&Path>,
    registry: &Registry,
    generated: &GeneratedArtifacts,
) -> Result<(), QualityError> {
    let (bindings, report) = output.map_or_else(
        || {
            (
                root.join(&registry.bindings_path),
                root.join(&registry.report_path),
            )
        },
        |directory| {
            (
                directory.join("quality-assurance-bindings.tsv"),
                directory.join("quality-assurance-status.md"),
            )
        },
    );
    for path in [&bindings, &report] {
        let parent = path
            .parent()
            .expect("generated quality paths always have a parent");
        fs::create_dir_all(parent).map_err(|error| {
            QualityError::from_diagnostic(Diagnostic::new(
                "QA-GENERATE-WRITE",
                "",
                path.to_string_lossy(),
                format!("cannot create output directory: {error}"),
            ))
        })?;
    }
    fs::write(&bindings, &generated.bindings).map_err(|error| {
        QualityError::from_diagnostic(Diagnostic::new(
            "QA-GENERATE-WRITE",
            "",
            bindings.to_string_lossy(),
            format!("cannot write binding output: {error}"),
        ))
    })?;
    fs::write(&report, &generated.report).map_err(|error| {
        QualityError::from_diagnostic(Diagnostic::new(
            "QA-GENERATE-WRITE",
            "",
            report.to_string_lossy(),
            format!("cannot write report output: {error}"),
        ))
    })
}

/// Validate and regenerate live bindings/report, optionally into a clean output directory.
///
/// # Errors
///
/// Returns stable diagnostics when validation, generation, or output writing fails.
pub fn generate_repository(root: &Path, output: Option<&Path>) -> Result<(), QualityError> {
    let registry = parse_registry(root, LIVE_REGISTRY)?;
    let generated = generate_from_registry(root, &registry)?;
    write_generated(root, output, &registry, &generated)
}

fn materialize_fixture_results(root: &Path, registry: &mut Registry) -> Result<(), QualityError> {
    for control in &mut registry.controls {
        let Some(result) = &mut control.result else {
            continue;
        };
        result.source_digest = digest_paths(root, &result.source_paths)?;
        result.tool_digest = digest_tool_authority(root, result)?;
        result.configuration_digest = digest_paths(root, &result.configuration_paths)?;
        result.scope_digest = digest_bytes(result.scope.as_bytes());
        result.result_digest = file_digest(root, &result.path)?;
    }
    Ok(())
}

#[derive(Clone, Debug)]
struct FixtureCase {
    id: String,
    polarity: String,
    mutation: String,
    expected_diagnostic: String,
    expected_count: usize,
}

fn parse_fixture_cases(text: &str) -> Result<Vec<FixtureCase>, QualityError> {
    let mut cases = Vec::new();
    for (index, line) in text.lines().enumerate() {
        if index == 0 {
            if line
                != "schema\tcase_id\tpolarity\tmutation\texpected_diagnostic\texpected_count\tcovers"
            {
                return Err(QualityError::from_diagnostic(Diagnostic::new(
                    "QA-FIXTURE-MANIFEST",
                    "",
                    FIXTURE_CASES,
                    "fixture manifest header is not frozen v1 shape",
                )));
            }
            continue;
        }
        let fields = line.split('\t').collect::<Vec<_>>();
        if fields.len() != 7 || fields[0] != "quality-assurance-fixture-case/v1" {
            return Err(QualityError::from_diagnostic(Diagnostic::new(
                "QA-FIXTURE-MANIFEST",
                "",
                FIXTURE_CASES,
                format!("fixture row {} is not frozen v1 shape", index + 1),
            )));
        }
        let expected_count = fields[5].parse::<usize>().map_err(|_| {
            QualityError::from_diagnostic(Diagnostic::new(
                "QA-FIXTURE-MANIFEST",
                "",
                FIXTURE_CASES,
                format!("fixture row {} has invalid cardinality", index + 1),
            ))
        })?;
        cases.push(FixtureCase {
            id: fields[1].to_owned(),
            polarity: fields[2].to_owned(),
            mutation: fields[3].to_owned(),
            expected_diagnostic: fields[4].to_owned(),
            expected_count,
        });
    }
    Ok(cases)
}

fn control_mut<'a>(registry: &'a mut Registry, id: &str) -> &'a mut Control {
    let position = registry
        .controls
        .iter()
        .position(|control| control.id == id)
        .unwrap_or(0);
    &mut registry.controls[position]
}

fn schema_mutation(base: &str, mutation: &str) -> Result<Option<String>, QualityError> {
    let mut value = toml::from_str::<toml::Value>(base).map_err(|_| {
        QualityError::from_diagnostic(Diagnostic::new(
            "QA-FIXTURE-PARSE",
            "",
            FIXTURE_REGISTRY,
            "cannot parse positive fixture as generic TOML",
        ))
    })?;
    let root = value
        .as_table_mut()
        .expect("parsed TOML documents always have a table root");
    let controls = root.get_mut("controls").and_then(toml::Value::as_array_mut);
    match mutation {
        "missing-required-name" => {
            if let Some(control) = controls.and_then(|items| items.first_mut()) {
                if let Some(table) = control.as_table_mut() {
                    table.remove("name");
                }
            }
        }
        "unknown-implementation-state" => {
            set_value_string(controls, 0, "implementation_state", "unknown");
        }
        "timestamp-field" => {
            root.insert(
                "timestamp".to_owned(),
                toml::Value::String("2026-07-22T00:00:00Z".to_owned()),
            );
        }
        "host-field" => {
            root.insert(
                "host".to_owned(),
                toml::Value::String("builder.example".to_owned()),
            );
        }
        "unknown-assurance-class" => {
            set_value_string(controls, 0, "assurance_class", "unknown");
        }
        "coverage-valid-schema" => {}
        "coverage-missing-result" => {
            let result = controls
                .expect("positive fixture controls")
                .get_mut(2)
                .expect("positive result control")
                .as_table_mut()
                .expect("positive result control table")
                .get_mut("result")
                .expect("positive result authority")
                .as_table_mut()
                .expect("positive result authority table");
            result.insert(
                "path".to_owned(),
                toml::Value::String("missing-quality-authority".to_owned()),
            );
        }
        _ => return Ok(None),
    }
    Ok(Some(
        toml::to_string(&value).expect("parsed TOML values remain serializable"),
    ))
}

fn set_value_string(
    controls: Option<&mut Vec<toml::Value>>,
    index: usize,
    field: &str,
    value: &str,
) {
    if let Some(table) = controls
        .and_then(|items| items.get_mut(index))
        .and_then(toml::Value::as_table_mut)
    {
        table.insert(field.to_owned(), toml::Value::String(value.to_owned()));
    }
}

fn apply_fixture_mutation(registry: &mut Registry, mutation: &str) {
    match mutation {
        "malformed-control-id" => "QA-9".clone_into(&mut control_mut(registry, "QA-9001").id),
        "retired-id-reuse" | "terminal-retired-id-reuse" => {
            registry.retired_ids.push("QA-9001".to_owned());
        }
        "duplicate-control" => {
            let duplicate = control_mut(registry, "QA-9001").clone();
            registry.controls.push(duplicate);
        }
        "duplicate-binding" => {
            let control = control_mut(registry, "QA-9003");
            control
                .implementation_paths
                .push("fixtures/quality/v1/repository/implementation.rs".to_owned());
        }
        "unaccounted-tool" => control_mut(registry, "QA-9002").accounts.clear(),
        "unaccounted-task" => {
            control_mut(registry, "QA-9003")
                .accounts
                .retain(|account| !account.starts_with("task:"));
        }
        "unaccounted-script" => {
            control_mut(registry, "QA-9003")
                .accounts
                .retain(|account| !account.starts_with("script:"));
        }
        "unaccounted-obligation" => control_mut(registry, "QA-9006").accounts.clear(),
        "pinned-only-implemented" => {
            control_mut(registry, "QA-9002").implementation_state =
                ImplementationState::Implemented;
        }
        "implemented-auto-focused" => control_mut(registry, "QA-9003").focused_command = None,
        "focused-auto-blocking" => {
            let control = control_mut(registry, "QA-9003");
            control.enforcement_state = EnforcementState::Blocking;
            control.gate_invocation = None;
            control.gate_omission_control = None;
        }
        "registration-auto-pass" => {
            control_mut(registry, "QA-9002").execution_verdict = ExecutionVerdict::Pass;
        }
        "historical-result-current" => {
            if let Some(result) = &mut control_mut(registry, "QA-9003").result {
                "xxh3-128:00000000000000000000000000000000".clone_into(&mut result.result_digest);
            }
        }
        "missing-implementation-path" => {
            control_mut(registry, "QA-9003").implementation_paths =
                vec!["fixtures/quality/v1/repository/missing.rs".to_owned()];
        }
        "stale-source-digest" => {
            if let Some(result) = &mut control_mut(registry, "QA-9003").result {
                "xxh3-128:00000000000000000000000000000000".clone_into(&mut result.source_digest);
            }
        }
        "stale-tool-digest" => {
            if let Some(result) = &mut control_mut(registry, "QA-9003").result {
                "xxh3-128:00000000000000000000000000000000".clone_into(&mut result.tool_digest);
            }
        }
        "stale-scope-digest" => {
            if let Some(result) = &mut control_mut(registry, "QA-9003").result {
                "xxh3-128:00000000000000000000000000000000".clone_into(&mut result.scope_digest);
            }
        }
        "result-verdict-mismatch" => {
            control_mut(registry, "QA-9003").execution_verdict = ExecutionVerdict::Fail;
        }
        "missing-positive-control" => control_mut(registry, "QA-9003").positive_controls.clear(),
        "missing-negative-control" => control_mut(registry, "QA-9003").negative_controls.clear(),
        "omitted-gate-command" => {
            control_mut(registry, "QA-9006").gate_invocation = Some("missing-gate-lane".to_owned());
        }
        "blocked-without-discharge" => {
            control_mut(registry, "QA-9004").discharge_condition = None;
        }
        "retired-without-rationale" => {
            control_mut(registry, "QA-9005").retirement_rationale = None;
        }
        "ambiguous-owner" | "duplicate-related-owner" => {
            control_mut(registry, "QA-9003")
                .owner
                .additional_claimants
                .push("harden-spec-provenance-production-readiness".to_owned());
        }
        "active-owner-as-baseline" => {
            let owner = &mut control_mut(registry, "QA-9003").owner;
            owner.kind = OwnerKind::Baseline;
            owner.authority = String::from("openspec/changes/README.md");
        }
        "duplicate-evidence-credit" => {
            control_mut(registry, "QA-9002").evidence_credits =
                vec!["FIXTURE-CREDIT-9001".to_owned()];
        }
        "checkbox-only-implementation" => {
            let control = control_mut(registry, "QA-9003");
            control.implementation_paths = vec![
                "openspec/changes/archive/2026-08-02-add-buoy-quality-assurance-tracking/tasks.md"
                    .to_owned(),
            ];
        }
        "incomplete-replacement" => control_mut(registry, "QA-9006").replaces.clear(),
        "absolute-path" => {
            control_mut(registry, "QA-9001").documentation_paths = vec!["/tmp/qa".to_owned()];
        }
        "traversing-path" => {
            control_mut(registry, "QA-9001").documentation_paths = vec!["../qa".to_owned()];
        }
        "sampled-as-exhaustive" => {
            control_mut(registry, "QA-9007").assurance_class = AssuranceClass::BoundedExhaustive;
        }
        "coverage-as-proof" => {
            control_mut(registry, "QA-9010").assurance_class = AssuranceClass::FormallyProved;
        }
        "incomplete-formal-binding" => {
            if let Some(binding) = &mut control_mut(registry, "QA-9009").formal_binding {
                binding.verifier.clear();
            }
        }
        "missing-finite-model" => control_mut(registry, "QA-9008").finite_model = None,
        "verdict-with-stale-evidence" => {
            control_mut(registry, "QA-9003").evidence_state = EvidenceState::Stale;
        }
        "unknown-tool-binding" => {
            control_mut(registry, "QA-9002")
                .tool_bindings
                .push("unknown-fixture-tool".to_owned());
        }
        "available-without-identity" => {
            let control = control_mut(registry, "QA-9002");
            control.tool_bindings.clear();
            control.configuration_paths.clear();
        }
        "not-run-with-result" => {
            let result = control_mut(registry, "QA-9003").result.clone();
            control_mut(registry, "QA-9001").result = result;
        }
        "retired-lifecycle-with-planned-state" => {
            control_mut(registry, "QA-9001").lifecycle = Lifecycle::Retired;
        }
        "missing-assurance-scope" => control_mut(registry, "QA-9001").exclusions.clear(),
        "current-without-evidence" => control_mut(registry, "QA-9002").evidence_paths.clear(),
        "wrong-schema-id" => "quality-assurance-controls/v0".clone_into(&mut registry.schema),
        "wrong-digest-policy" => "sha256".clone_into(&mut registry.digest_policy),
        "wrong-final-command" => "focused command".clone_into(&mut registry.final_command),
        "invalid-registry-path" => registry.report_path.clear(),
        "unreadable-gate-source" => "missing-gate.json".clone_into(&mut registry.gate_source),
        "source-text-gate" => "devenv.nix".clone_into(&mut registry.gate_source),
        "duplicate-disposition" => {
            registry
                .non_applicable
                .push(registry.non_applicable[0].clone());
        }
        "empty-disposition-rationale" => registry.non_applicable[0].rationale.clear(),
        _ => {}
    }
}

fn copy_authorities(source: &Path, target: &Path, registry: &Registry) -> Result<(), QualityError> {
    let mut paths = BTreeSet::new();
    paths.insert(registry.gate_source.clone());
    for control in &registry.controls {
        paths.extend(binding_paths(control).into_keys());
    }
    for relative in paths {
        let source_path = source.join(&relative);
        let target_path = target.join(&relative);
        let parent = target_path
            .parent()
            .expect("alternate-root authority paths always have a parent");
        fs::create_dir_all(parent).map_err(|error| {
            QualityError::from_diagnostic(Diagnostic::new(
                "QA-FIXTURE-ROOT",
                "",
                &relative,
                format!("cannot create alternate-root fixture directory: {error}"),
            ))
        })?;
        fs::copy(&source_path, &target_path).map_err(|error| {
            QualityError::from_diagnostic(Diagnostic::new(
                "QA-FIXTURE-ROOT",
                "",
                &relative,
                format!("cannot copy alternate-root authority: {error}"),
            ))
        })?;
    }
    Ok(())
}

fn remove_fixture_root(path: &Path, action: &str) -> Result<(), QualityError> {
    fs::remove_dir_all(path).map_err(|error| {
        QualityError::from_diagnostic(Diagnostic::new(
            "QA-FIXTURE-ROOT",
            "",
            path.to_string_lossy(),
            format!("cannot {action} alternate-root fixture: {error}"),
        ))
    })
}

fn run_positive_fixture_case(
    root: &Path,
    base: &Registry,
    mutation: &str,
    baseline: &GeneratedArtifacts,
) -> Result<(), QualityError> {
    match mutation {
        "none" => generate_from_registry(root, base).map(|_| ()),
        "reverse-controls" => {
            let mut registry = base.clone();
            registry.controls.reverse();
            let generated = generate_from_registry(root, &registry)?;
            if generated == *baseline {
                Ok(())
            } else {
                Err(QualityError::from_diagnostic(Diagnostic::new(
                    "QA-DETERMINISM-ORDER",
                    "",
                    FIXTURE_REGISTRY,
                    "control discovery order changed generated bytes",
                )))
            }
        }
        "alternate-root" | "alternate-root-cleanup-control" => {
            let temporary = std::env::temp_dir().join(format!(
                "buoy-quality-root-{}-{:?}",
                std::process::id(),
                std::thread::current().id()
            ));
            if temporary.exists() {
                remove_fixture_root(&temporary, "clear")?;
            }
            copy_authorities(root, &temporary, base)?;
            let generated = generate_from_registry(&temporary, base);
            if mutation == "alternate-root-cleanup-control" {
                remove_fixture_root(&temporary, "preclean")
                    .expect("coverage-control preclean must succeed");
            }
            remove_fixture_root(&temporary, "clean")?;
            if generated? == *baseline {
                Ok(())
            } else {
                Err(QualityError::from_diagnostic(Diagnostic::new(
                    "QA-DETERMINISM-ROOT",
                    "",
                    FIXTURE_REGISTRY,
                    "absolute repository root changed generated bytes",
                )))
            }
        }
        _ => Err(QualityError::from_diagnostic(Diagnostic::new(
            "QA-FIXTURE-MUTATION",
            "",
            mutation,
            "unknown positive fixture mutation",
        ))),
    }
}

fn run_negative_fixture_case(
    root: &Path,
    base_text: &str,
    base: &Registry,
    mutation: &str,
    baseline: &GeneratedArtifacts,
) -> Result<QualityError, QualityError> {
    if let Some(text) = schema_mutation(base_text, mutation)? {
        return parse_registry_text(&text, FIXTURE_REGISTRY)
            .and_then(|mut registry| {
                materialize_fixture_results(root, &mut registry)?;
                generate_from_registry(root, &registry).map(|_| registry)
            })
            .map_or_else(Ok, |_| {
                Err(QualityError::from_diagnostic(Diagnostic::new(
                    "QA-FIXTURE-POLARITY",
                    "",
                    mutation,
                    "negative schema fixture unexpectedly passed",
                )))
            });
    }
    if mutation == "report-drift" {
        let drift = format!("{}hand edit\n", baseline.report);
        return compare_text(&baseline.report, &drift, "QA-REPORT-DRIFT");
    }
    if mutation == "bindings-drift" {
        let drift = format!("{}hand edit\n", baseline.bindings);
        return compare_text(&baseline.bindings, &drift, "QA-BINDINGS-DRIFT");
    }
    let mut registry = base.clone();
    apply_fixture_mutation(&mut registry, mutation);
    match generate_from_registry(root, &registry) {
        Ok(_) => Err(QualityError::from_diagnostic(Diagnostic::new(
            "QA-FIXTURE-POLARITY",
            "",
            mutation,
            "negative registry fixture unexpectedly passed",
        ))),
        Err(error) => Ok(error),
    }
}

fn compare_text(
    expected: &str,
    actual: &str,
    code: &'static str,
) -> Result<QualityError, QualityError> {
    if expected == actual {
        Err(QualityError::from_diagnostic(Diagnostic::new(
            "QA-FIXTURE-POLARITY",
            "",
            code,
            "drift fixture unexpectedly matched",
        )))
    } else {
        Ok(QualityError::from_diagnostic(Diagnostic::new(
            code,
            "",
            "fixture-generated-artifact",
            "fixture generated artifact differs from expected bytes",
        )))
    }
}

#[derive(Clone, Debug)]
struct FixtureExecution {
    case: FixtureCase,
    diagnostic: String,
    count: usize,
}

fn execute_fixture_cases_with_results(
    root: &Path,
    base_text: &str,
    base: &Registry,
    baseline: &GeneratedArtifacts,
    cases: Vec<FixtureCase>,
) -> Result<Vec<FixtureExecution>, QualityError> {
    let mut ids = BTreeSet::new();
    let mut executions = Vec::new();
    for case in cases {
        if !ids.insert(case.id.clone()) {
            return Err(QualityError::from_diagnostic(Diagnostic::new(
                "QA-FIXTURE-MANIFEST",
                "",
                &case.id,
                "fixture case identity is duplicated",
            )));
        }
        if case.polarity == "positive" {
            run_positive_fixture_case(root, base, &case.mutation, baseline)?;
            executions.push(FixtureExecution {
                case,
                diagnostic: "-".to_owned(),
                count: 0,
            });
            continue;
        }
        if case.polarity != "negative" {
            return Err(QualityError::from_diagnostic(Diagnostic::new(
                "QA-FIXTURE-MANIFEST",
                "",
                &case.id,
                "fixture polarity must be positive or negative",
            )));
        }
        let error = run_negative_fixture_case(root, base_text, base, &case.mutation, baseline)?;
        if error.diagnostics.len() != case.expected_count
            || error.code_count(&case.expected_diagnostic) != case.expected_count
        {
            return Err(QualityError::from_diagnostic(Diagnostic::new(
                "QA-FIXTURE-DIAGNOSTIC",
                "",
                &case.id,
                format!(
                    "expected {} x {}, received: {}",
                    case.expected_count, case.expected_diagnostic, error
                ),
            )));
        }
        let count = error.diagnostics.len();
        executions.push(FixtureExecution {
            diagnostic: case.expected_diagnostic.clone(),
            case,
            count,
        });
    }
    Ok(executions)
}

fn execute_fixture_cases(
    root: &Path,
    base_text: &str,
    base: &Registry,
    baseline: &GeneratedArtifacts,
    cases: Vec<FixtureCase>,
) -> Result<(), QualityError> {
    execute_fixture_cases_with_results(root, base_text, base, baseline, cases).map(drop)
}

#[allow(clippy::assigning_clones, clippy::too_many_lines)]
fn coverage_controls(root: &Path, base_text: &str, base: &Registry, baseline: &GeneratedArtifacts) {
    let formatted = QualityError::from_diagnostics(vec![
        Diagnostic::new("QA-CONTROL-B", "QA-9999", "b", "second"),
        Diagnostic::new("QA-CONTROL-A", "QA-9998", "a", "first"),
        Diagnostic::new("QA-CONTROL-A", "QA-9998", "a", "first"),
    ])
    .to_string();
    drop(formatted);
    expected_control_failure(read_text(
        root,
        "missing-quality-control",
        "QA-CONTROL-READ",
    ));
    expected_control_failure(parse_registry(root, "missing-quality-control"));
    expected_control_failure(generated_artifacts(&root.join("missing-quality-root")));
    expected_control_failure(check_repository(&root.join("missing-quality-root")));
    expected_control_failure(generate_repository(
        &root.join("missing-quality-root"),
        None,
    ));
    let _ = valid_control_id("XX-0000");
    let _ = valid_control_id("QA-ABCD");
    let _ = validate_relative_path("quality\\control");

    let mut synthetic_without_inventory = base.clone();
    synthetic_without_inventory.synthetic_inventory = None;
    expected_control_failure(discover_inventory(root, &synthetic_without_inventory));
    let mut live_with_inventory = base.clone();
    live_with_inventory.mode = RegistryMode::Live;
    expected_control_failure(discover_inventory(root, &live_with_inventory));

    let scratch = std::env::temp_dir().join(format!(
        "buoy-quality-coverage-controls-{}",
        std::process::id()
    ));
    drop(fs::remove_dir_all(&scratch));
    for directory in ["assurance/v1", "fixtures/devenv/v1", "nix/devenv"] {
        fs::create_dir_all(scratch.join(directory))
            .expect("quality coverage-control directory must be writable");
    }
    let mut discovered = base.clone();
    discovered.mode = RegistryMode::Live;
    discovered.synthetic_inventory = None;
    expected_control_failure(discover_inventory(&scratch, &discovered));
    expected_control_failure(validate_registry(&scratch, &discovered));
    fs::write(scratch.join("assurance/v1/toolchain.toml"), "[")
        .expect("quality coverage-control toolchain must be writable");
    expected_control_failure(discover_inventory(&scratch, &discovered));
    fs::write(
        scratch.join("assurance/v1/toolchain.toml"),
        "[[tool]]\nid = \"fixture\"\nrole = \"qa-role\"\n",
    )
    .expect("quality coverage-control toolchain must be writable");
    expected_control_failure(discover_inventory(&scratch, &discovered));
    fs::write(scratch.join("nix/devenv/workflow-policy.toml"), "[")
        .expect("quality coverage-control workflow policy must be writable");
    expected_control_failure(discover_inventory(&scratch, &discovered));
    fs::write(
        scratch.join("nix/devenv/workflow-policy.toml"),
        "[[script_owner]]\nscripts = [\"buoy-control\"]\n",
    )
    .expect("quality coverage-control workflow policy must be writable");
    expected_control_failure(discover_inventory(&scratch, &discovered));
    fs::write(scratch.join("fixtures/devenv/v1/valid-tasks.json"), "[")
        .expect("quality coverage-control tasks must be writable");
    expected_control_failure(discover_inventory(&scratch, &discovered));
    fs::write(
        scratch.join("fixtures/devenv/v1/valid-tasks.json"),
        "[{\"name\":\"buoy:control\",\"after\":[],\"before\":[]}]\n",
    )
    .expect("quality coverage-control tasks must be writable");
    expected_control_failure(discover_inventory(&scratch, &discovered));
    fs::write(scratch.join("assurance/v1/phase-handoffs.toml"), "[")
        .expect("quality coverage-control handoffs must be writable");
    expected_control_failure(discover_inventory(&scratch, &discovered));
    for handoffs in [
        "",
        "[[phase]]\nrequires = []\n",
        "[[phase]]\nid = \"fixture\"\n",
        "[[phase]]\nid = \"fixture\"\nrequires = [\"requirement\", 1]\n",
    ] {
        fs::write(scratch.join("assurance/v1/phase-handoffs.toml"), handoffs)
            .expect("quality coverage-control handoffs must be writable");
        drop(
            discover_inventory(&scratch, &discovered).expect("minimal discovery control must pass"),
        );
    }

    let missing = vec!["missing-quality-authority".to_owned()];
    expected_control_failure(digest_paths(root, &missing));
    expected_control_failure(file_digest(root, "missing-quality-authority"));
    let mut missing_binding = base.clone();
    control_mut(&mut missing_binding, "QA-9001")
        .documentation_paths
        .push("missing-quality-authority".to_owned());
    expected_control_failure(render_bindings(root, &missing_binding));
    expected_control_failure(render_validated_registry(root, &missing_binding));
    let mut result = control_mut(&mut base.clone(), "QA-9003")
        .result
        .clone()
        .expect("positive fixture result authority");
    result.tool_paths = missing.clone();
    expected_control_failure(digest_tool_authority(root, &result));

    let mut record = control_mut(&mut base.clone(), "QA-9003")
        .result
        .clone()
        .expect("positive fixture result authority");
    record.path = "result.tsv".to_owned();
    let _ = result_record_matches(&scratch, &record, ExecutionVerdict::Pass);
    fs::write(scratch.join(&record.path), "wrong\n")
        .expect("quality result control must be writable");
    let _ = result_record_matches(&scratch, &record, ExecutionVerdict::Pass);
    fs::write(scratch.join(&record.path), "schema\tverdict\tscope\n")
        .expect("quality result control must be writable");
    let _ = result_record_matches(&scratch, &record, ExecutionVerdict::Pass);
    fs::write(
        scratch.join(&record.path),
        format!(
            "schema\tverdict\tscope\nquality-assurance-result/v1\tpass\t{}\nextra\n",
            record.scope
        ),
    )
    .expect("quality result control must be writable");
    let _ = result_record_matches(&scratch, &record, ExecutionVerdict::Pass);
    for row in [
        "too\tfew",
        "wrong-schema\tpass\tSynthetic implemented focused control with a current pass.",
        "quality-assurance-result/v1\tpass\twrong-scope",
    ] {
        fs::write(
            scratch.join(&record.path),
            format!("schema\tverdict\tscope\n{row}\n"),
        )
        .expect("quality result control must be writable");
        let _ = result_record_matches(&scratch, &record, ExecutionVerdict::Pass);
    }

    let current = control_mut(&mut base.clone(), "QA-9003")
        .result
        .clone()
        .expect("positive fixture result authority");
    let _ = result_is_current(root, &current);
    let mut stale = current.clone();
    stale.configuration_digest = "xxh3-128:00000000000000000000000000000000".to_owned();
    let _ = result_is_current(root, &stale);
    let mut stale = current.clone();
    stale.result_digest = "xxh3-128:00000000000000000000000000000000".to_owned();
    let _ = result_is_current(root, &stale);
    let mut stale = current.clone();
    stale.scope.clear();
    stale.scope_digest = digest_bytes(b"");
    let _ = result_is_current(root, &stale);

    let mut materialize = base.clone();
    control_mut(&mut materialize, "QA-9003")
        .result
        .as_mut()
        .expect("positive fixture result authority")
        .source_paths = missing.clone();
    expected_control_failure(materialize_fixture_results(root, &mut materialize));
    let mut materialize = base.clone();
    control_mut(&mut materialize, "QA-9003")
        .result
        .as_mut()
        .expect("positive fixture result authority")
        .tool_paths = missing.clone();
    expected_control_failure(materialize_fixture_results(root, &mut materialize));
    let mut materialize = base.clone();
    control_mut(&mut materialize, "QA-9003")
        .result
        .as_mut()
        .expect("positive fixture result authority")
        .configuration_paths = missing.clone();
    expected_control_failure(materialize_fixture_results(root, &mut materialize));
    let mut materialize = base.clone();
    control_mut(&mut materialize, "QA-9003")
        .result
        .as_mut()
        .expect("positive fixture result authority")
        .path = "missing-quality-authority".to_owned();
    expected_control_failure(materialize_fixture_results(root, &mut materialize));

    let mut assurance = base.clone();
    control_mut(&mut assurance, "QA-9008")
        .finite_model
        .as_mut()
        .expect("positive finite model")
        .id
        .clear();
    validate_assurance(&assurance, &mut Vec::new());
    let mut assurance = base.clone();
    control_mut(&mut assurance, "QA-9008")
        .finite_model
        .as_mut()
        .expect("positive finite model")
        .path
        .clear();
    validate_assurance(&assurance, &mut Vec::new());
    let mut assurance = base.clone();
    control_mut(&mut assurance, "QA-9008")
        .finite_model
        .as_mut()
        .expect("positive finite model")
        .bounds
        .clear();
    validate_assurance(&assurance, &mut Vec::new());
    let mut assurance = base.clone();
    control_mut(&mut assurance, "QA-9008")
        .finite_model
        .as_mut()
        .expect("positive finite model")
        .transitions
        .clear();
    validate_assurance(&assurance, &mut Vec::new());
    let mut assurance = base.clone();
    control_mut(&mut assurance, "QA-9008")
        .finite_model
        .as_mut()
        .expect("positive finite model")
        .invariants
        .clear();
    validate_assurance(&assurance, &mut Vec::new());

    let formal = control_mut(&mut base.clone(), "QA-9009")
        .formal_binding
        .clone()
        .expect("positive formal binding");
    let _ = complete_formal_binding(&formal);
    let mut incomplete = formal.clone();
    incomplete.implementation_representation.clear();
    let _ = complete_formal_binding(&incomplete);
    let mut incomplete = formal.clone();
    incomplete.proof_source.clear();
    let _ = complete_formal_binding(&incomplete);
    let mut incomplete = formal.clone();
    incomplete.solver.clear();
    let _ = complete_formal_binding(&incomplete);
    let mut incomplete = formal.clone();
    incomplete.configuration.clear();
    let _ = complete_formal_binding(&incomplete);
    let mut incomplete = formal.clone();
    incomplete.correspondence.clear();
    let _ = complete_formal_binding(&incomplete);
    let mut incomplete = formal.clone();
    incomplete.negative_control.clear();
    let _ = complete_formal_binding(&incomplete);
    let mut incomplete = formal.clone();
    incomplete.scope.clear();
    let _ = complete_formal_binding(&incomplete);
    let mut incomplete = formal.clone();
    incomplete.assumptions.clear();
    let _ = complete_formal_binding(&incomplete);
    let mut incomplete = formal;
    incomplete.exclusions.clear();
    let _ = complete_formal_binding(&incomplete);

    let mut assurance = base.clone();
    control_mut(&mut assurance, "QA-9001")
        .trust_statement
        .clear();
    validate_assurance(&assurance, &mut Vec::new());
    let mut assurance = base.clone();
    control_mut(&mut assurance, "QA-9001").non_claims.clear();
    validate_assurance(&assurance, &mut Vec::new());
    let mut evidence = base.clone();
    control_mut(&mut evidence, "QA-9001").evidence_state = EvidenceState::Current;
    validate_evidence_state(&evidence, &mut Vec::new());

    let inventory = base
        .synthetic_inventory
        .as_ref()
        .expect("positive synthetic inventory");
    let mut states = base.clone();
    control_mut(&mut states, "QA-9002").tool_bindings.clear();
    validate_states(inventory, &states, &mut Vec::new());
    let mut states = base.clone();
    control_mut(&mut states, "QA-9004").blocker = None;
    validate_states(inventory, &states, &mut Vec::new());
    let mut states = base.clone();
    let retired = control_mut(&mut states, "QA-9005");
    retired.replacement = None;
    retired.terminal_retirement = false;
    validate_states(inventory, &states, &mut Vec::new());
    let mut states = base.clone();
    let retired = control_mut(&mut states, "QA-9005");
    retired.replacement = None;
    retired.terminal_retirement = true;
    validate_states(inventory, &states, &mut Vec::new());
    let mut states = base.clone();
    control_mut(&mut states, "QA-9006").focused_command = None;
    validate_states(inventory, &states, &mut Vec::new());
    let mut states = base.clone();
    control_mut(&mut states, "QA-9006").gate_omission_control = None;
    validate_states(inventory, &states, &mut Vec::new());

    let mut ownership = base.clone();
    control_mut(&mut ownership, "QA-9001")
        .owner
        .authority
        .clear();
    validate_ownership(&ownership, &mut Vec::new());
    let mut ownership = base.clone();
    let owner = &mut control_mut(&mut ownership, "QA-9001").owner;
    owner.related_changes.push(owner.authority.clone());
    validate_ownership(&ownership, &mut Vec::new());

    fs::write(scratch.join("drift.txt"), "actual").expect("quality drift control must be writable");
    expected_control_failure(compare_artifact(
        &scratch,
        "drift.txt",
        "expected",
        "QA-CONTROL-DRIFT",
    ));
    expected_control_failure(compare_artifact(
        &scratch,
        "missing-quality-artifact",
        "expected",
        "QA-CONTROL-DRIFT",
    ));
    let invalid_live_root = scratch.join("invalid-live-check");
    let invalid_live_registry = invalid_live_root.join(LIVE_REGISTRY);
    fs::create_dir_all(
        invalid_live_registry
            .parent()
            .expect("invalid live registry path has a parent"),
    )
    .expect("invalid live registry parent must be writable");
    fs::copy(root.join(LIVE_REGISTRY), &invalid_live_registry)
        .expect("invalid live registry control must copy");
    expected_control_failure(generated_artifacts(&invalid_live_root));
    expected_control_failure(check_repository(&invalid_live_root));
    expected_control_failure(generate_repository(&invalid_live_root, None));
    let live_root = scratch.join("live-check");
    let live_registry =
        parse_registry(root, LIVE_REGISTRY).expect("live registry control must parse");
    copy_authorities(root, &live_root, &live_registry)
        .expect("live registry authorities must copy for drift control");
    for relative in [
        LIVE_REGISTRY,
        "assurance/v1/toolchain.toml",
        "assurance/v1/phase-handoffs.toml",
        &live_registry.bindings_path,
        &live_registry.report_path,
    ] {
        let target = live_root.join(relative);
        fs::create_dir_all(target.parent().expect("live control path has a parent"))
            .expect("live control parent must be writable");
        fs::copy(root.join(relative), target).expect("live control authority must copy");
    }
    fs::write(
        live_root.join(&live_registry.bindings_path),
        "stale bindings\n",
    )
    .expect("live binding drift control must be writable");
    expected_control_failure(check_repository(&live_root));
    let mut write_registry = base.clone();
    write_registry.bindings_path = "write-success/bindings.tsv".to_owned();
    write_registry.report_path = "write-success/report.md".to_owned();
    write_generated(&scratch, None, &write_registry, baseline)
        .expect("default quality output control must pass");
    let blocked_parent = scratch.join("blocked-parent");
    fs::write(&blocked_parent, "file").expect("quality blocked parent must be writable");
    expected_control_failure(write_generated(
        &scratch,
        Some(&blocked_parent),
        base,
        baseline,
    ));
    let binding_failure = scratch.join("binding-failure");
    fs::create_dir_all(binding_failure.join("quality-assurance-bindings.tsv"))
        .expect("quality binding failure directory must be writable");
    expected_control_failure(write_generated(
        &scratch,
        Some(&binding_failure),
        base,
        baseline,
    ));
    let report_failure = scratch.join("report-failure");
    fs::create_dir_all(report_failure.join("quality-assurance-status.md"))
        .expect("quality report failure directory must be writable");
    expected_control_failure(write_generated(
        &scratch,
        Some(&report_failure),
        base,
        baseline,
    ));

    expected_control_failure(parse_fixture_cases("wrong-header\n"));
    expected_control_failure(parse_fixture_cases(
        "schema\tcase_id\tpolarity\tmutation\texpected_diagnostic\texpected_count\tcovers\nbad\n",
    ));
    expected_control_failure(parse_fixture_cases(
        "schema\tcase_id\tpolarity\tmutation\texpected_diagnostic\texpected_count\tcovers\nwrong-schema\tid\tnegative\tnone\tQA\t1\tcontrol\n",
    ));
    expected_control_failure(parse_fixture_cases(
        "schema\tcase_id\tpolarity\tmutation\texpected_diagnostic\texpected_count\tcovers\nquality-assurance-fixture-case/v1\tid\tnegative\tnone\tQA\tnot-a-number\tcontrol\n",
    ));
    expected_control_failure(schema_mutation("not TOML", "none"));
    drop(
        schema_mutation("schema = \"control\"\n", "missing-required-name")
            .expect("control TOML must parse"),
    );
    drop(
        schema_mutation("controls = [1]\n", "missing-required-name")
            .expect("control TOML must parse"),
    );
    set_value_string(None, 0, "unused", "unused");
    let mut unknown = base.clone();
    apply_fixture_mutation(&mut unknown, "unknown-coverage-mutation");
    let mut missing_result = base.clone();
    control_mut(&mut missing_result, "QA-9003").result = None;
    for mutation in [
        "historical-result-current",
        "stale-source-digest",
        "stale-tool-digest",
        "stale-scope-digest",
    ] {
        apply_fixture_mutation(&mut missing_result, mutation);
    }
    let mut missing_formal = base.clone();
    control_mut(&mut missing_formal, "QA-9009").formal_binding = None;
    apply_fixture_mutation(&mut missing_formal, "incomplete-formal-binding");

    let copy_parent = scratch.join("copy-parent");
    fs::write(&copy_parent, "file").expect("quality copy parent must be writable");
    let mut copy_registry = base.clone();
    copy_registry.controls.clear();
    expected_control_failure(copy_authorities(root, &copy_parent, &copy_registry));
    let copy_target = scratch.join("copy-missing");
    let mut copy_registry = copy_registry.clone();
    copy_registry.gate_source = "missing-quality-authority".to_owned();
    expected_control_failure(copy_authorities(root, &copy_target, &copy_registry));
    expected_control_failure(remove_fixture_root(
        &scratch.join("missing-remove-root"),
        "remove",
    ));

    assert!(
        resolved_tasks_from(None, root)
            .expect("the reviewed fallback task fixture parses")
            .len()
            > 1
    );
    let unknown_dependency = vec![ResolvedTask {
        name: String::from("buoy:control"),
        after: vec![String::from("missing@succeeded")],
        before: Vec::new(),
    }];
    expected_control_failure(resolved_task_prerequisites(&unknown_dependency).map(|_| ()));
    let unknown_dependent = vec![ResolvedTask {
        name: String::from("buoy:control"),
        after: Vec::new(),
        before: vec![String::from("missing")],
    }];
    expected_control_failure(resolved_task_prerequisites(&unknown_dependent).map(|_| ()));
    let disconnected = BTreeMap::from([
        (String::from("buoy:control"), BTreeSet::new()),
        (String::from("devenv:enterTest"), BTreeSet::new()),
    ]);
    assert!(!task_reaches(
        &disconnected,
        "buoy:control",
        "devenv:enterTest"
    ));
    assert!(!task_reaches(
        &BTreeMap::new(),
        "buoy:control",
        "devenv:enterTest"
    ));
    let mut live_registry = base.clone();
    live_registry.mode = RegistryMode::Live;
    live_registry.synthetic_inventory = None;
    expected_control_failure(
        discover_inventory_from(
            root,
            &live_registry,
            Some(scratch.join("missing-task-source.json")),
        )
        .map(|_| ()),
    );
    let gate_control = scratch.join("unknown-dependency-tasks.json");
    fs::write(
        &gate_control,
        "[{\"name\":\"buoy:control\",\"after\":[\"missing\"],\"before\":[]}]",
    )
    .expect("gate control task file must be writable");
    let mut gate_registry = base.clone();
    gate_registry.gate_source = gate_control.to_string_lossy().into_owned();
    let mut gate_diagnostics = Vec::new();
    validate_gate(root, &gate_registry, &mut gate_diagnostics);
    assert!(
        gate_diagnostics
            .iter()
            .any(|diagnostic| diagnostic.code == "QA-GATE-SCHEMA"),
        "unknown-dependency gate control must fail closed"
    );

    let wrong_baseline = GeneratedArtifacts {
        bindings: String::new(),
        report: String::new(),
    };
    expected_control_failure(run_positive_fixture_case(
        root,
        base,
        "reverse-controls",
        &wrong_baseline,
    ));
    let mut invalid_base = base.clone();
    invalid_base.synthetic_inventory = None;
    expected_control_failure(run_positive_fixture_case(
        root,
        &invalid_base,
        "reverse-controls",
        baseline,
    ));
    let alternate = std::env::temp_dir().join(format!(
        "buoy-quality-root-{}-{:?}",
        std::process::id(),
        std::thread::current().id()
    ));
    drop(fs::remove_dir_all(&alternate));
    fs::write(&alternate, "file").expect("alternate-root file control must be writable");
    expected_control_failure(run_positive_fixture_case(
        root,
        base,
        "alternate-root",
        baseline,
    ));
    fs::remove_file(&alternate).expect("alternate-root file control must be removable");
    let mut copy_failure = base.clone();
    copy_failure.gate_source = "missing-quality-authority".to_owned();
    expected_control_failure(run_positive_fixture_case(
        root,
        &copy_failure,
        "alternate-root",
        baseline,
    ));
    drop(fs::remove_dir_all(&alternate));
    let mut validation_failure = base.clone();
    validation_failure.schema = "quality-assurance-controls/v0".to_owned();
    expected_control_failure(run_positive_fixture_case(
        root,
        &validation_failure,
        "alternate-root",
        baseline,
    ));
    expected_control_failure(run_positive_fixture_case(
        root,
        base,
        "alternate-root-cleanup-control",
        baseline,
    ));
    fs::create_dir_all(&alternate).expect("alternate-root coverage control must be writable");
    expected_control_failure(run_positive_fixture_case(
        root,
        base,
        "alternate-root",
        &wrong_baseline,
    ));
    expected_control_failure(run_positive_fixture_case(
        root,
        base,
        "unknown-positive",
        baseline,
    ));
    expected_control_failure(run_negative_fixture_case(
        root,
        "not TOML",
        base,
        "missing-required-name",
        baseline,
    ));
    drop(run_negative_fixture_case(
        root,
        base_text,
        base,
        "coverage-missing-result",
        baseline,
    ));
    expected_control_failure(run_negative_fixture_case(
        root,
        base_text,
        base,
        "coverage-valid-schema",
        baseline,
    ));
    expected_control_failure(run_negative_fixture_case(
        root,
        base_text,
        base,
        "unknown-coverage-mutation",
        baseline,
    ));
    expected_control_failure(compare_text("same", "same", "QA-CONTROL-DRIFT"));

    let positive_case = FixtureCase {
        id: "duplicate".to_owned(),
        polarity: "positive".to_owned(),
        mutation: "none".to_owned(),
        expected_diagnostic: "-".to_owned(),
        expected_count: 0,
    };
    expected_control_failure(execute_fixture_cases(
        root,
        base_text,
        base,
        baseline,
        vec![positive_case.clone(), positive_case],
    ));
    expected_control_failure(execute_fixture_cases(
        root,
        base_text,
        base,
        baseline,
        vec![FixtureCase {
            id: "polarity".to_owned(),
            polarity: "unknown".to_owned(),
            mutation: "none".to_owned(),
            expected_diagnostic: "-".to_owned(),
            expected_count: 0,
        }],
    ));
    expected_control_failure(execute_fixture_cases(
        root,
        base_text,
        base,
        baseline,
        vec![FixtureCase {
            id: "positive-error".to_owned(),
            polarity: "positive".to_owned(),
            mutation: "unknown-positive".to_owned(),
            expected_diagnostic: "-".to_owned(),
            expected_count: 0,
        }],
    ));
    expected_control_failure(execute_fixture_cases(
        root,
        base_text,
        base,
        baseline,
        vec![FixtureCase {
            id: "negative-error".to_owned(),
            polarity: "negative".to_owned(),
            mutation: "coverage-valid-schema".to_owned(),
            expected_diagnostic: "QA-FIXTURE-POLARITY".to_owned(),
            expected_count: 1,
        }],
    ));
    expected_control_failure(execute_fixture_cases(
        root,
        base_text,
        base,
        baseline,
        vec![FixtureCase {
            id: "diagnostic".to_owned(),
            polarity: "negative".to_owned(),
            mutation: "report-drift".to_owned(),
            expected_diagnostic: "QA-WRONG".to_owned(),
            expected_count: 1,
        }],
    ));
    expected_control_failure(execute_fixture_cases(
        root,
        base_text,
        base,
        baseline,
        vec![FixtureCase {
            id: "cardinality".to_owned(),
            polarity: "negative".to_owned(),
            mutation: "report-drift".to_owned(),
            expected_diagnostic: "QA-REPORT-DRIFT".to_owned(),
            expected_count: 2,
        }],
    ));

    let self_test_root = scratch.join("self-test-errors");
    expected_control_failure(self_test(&self_test_root));
    let self_test_registry = self_test_root.join(FIXTURE_REGISTRY);
    fs::create_dir_all(
        self_test_registry
            .parent()
            .expect("self-test registry path has a parent"),
    )
    .expect("self-test registry parent must be writable");
    fs::write(&self_test_registry, base_text).expect("self-test registry must be writable");
    expected_control_failure(self_test(&self_test_root));
    let cases_text =
        read_text(root, FIXTURE_CASES, "QA-CONTROL-READ").expect("fixture cases must be readable");
    let self_test_cases = self_test_root.join(FIXTURE_CASES);
    fs::write(&self_test_cases, &cases_text).expect("self-test cases must be writable");
    fs::write(&self_test_registry, "not TOML").expect("self-test registry must be writable");
    expected_control_failure(self_test(&self_test_root));
    fs::write(&self_test_registry, base_text).expect("self-test registry must be writable");
    expected_control_failure(self_test(&self_test_root));
    copy_authorities(root, &self_test_root, base).expect("self-test authority controls must copy");
    let wrong_schema = base_text.replacen(
        "schema = \"quality-assurance-controls/v1\"",
        "schema = \"quality-assurance-controls/v0\"",
        1,
    );
    fs::write(&self_test_registry, wrong_schema).expect("self-test registry must be writable");
    expected_control_failure(self_test(&self_test_root));
    fs::write(&self_test_registry, base_text).expect("self-test registry must be writable");
    fs::write(&self_test_cases, "wrong-header\n").expect("self-test cases must be writable");
    expected_control_failure(self_test(&self_test_root));
    fs::write(
        &self_test_cases,
        "schema\tcase_id\tpolarity\tmutation\texpected_diagnostic\texpected_count\tcovers\nquality-assurance-fixture-case/v1\terror\tpositive\tunknown-positive\t-\t0\tcontrol\n",
    )
    .expect("self-test cases must be writable");
    expected_control_failure(self_test(&self_test_root));
    fs::remove_dir_all(&scratch).expect("quality coverage-control cleanup must succeed");
}

fn execution_evidence_error(error: String) -> QualityError {
    QualityError::from_diagnostic(Diagnostic::new(
        "QA-EXECUTION-EVIDENCE",
        "",
        FIXTURE_EXECUTIONS,
        error,
    ))
}

/// Build the stable diagnostic for a fixture case that the parsed case list
/// carries but the raw manifest text cannot re-derive a source row for.
///
/// Extracted from [`render_fixture_execution_evidence`] so the otherwise
/// unreachable defensive path (both extractions are always derived from the
/// same manifest text) can be exercised directly and honestly.
fn missing_source_row_error(id: &str) -> QualityError {
    execution_evidence_error(format!("missing source row for {id}"))
}

/// Bind a set of local authority paths for execution evidence, wrapping the
/// shared library's error into the stable quality diagnostic.
///
/// Extracted from [`render_fixture_execution_evidence`] so the binding
/// failure path can be exercised directly against a genuinely missing path
/// without disturbing the real tool/configuration authorities it is
/// ordinarily bound against.
fn execution_tool_binding_digest(paths: &[&str]) -> Result<String, QualityError> {
    execution_evidence::path_set_binding(paths).map_err(execution_evidence_error)
}

/// Build one fixture-mutation execution-evidence record, wrapping the shared
/// library's error into the stable quality diagnostic.
///
/// Extracted from [`render_fixture_execution_evidence`] so the binding
/// failure path (an invalid tool/configuration digest) can be exercised
/// directly with a deliberately malformed digest.
#[allow(clippy::too_many_arguments)]
fn fixture_execution_record(
    index: usize,
    execution: &FixtureExecution,
    source: &str,
    tool_paths: &str,
    tool_digest: &str,
    configuration_paths: &str,
    configuration_digest: &str,
) -> Result<execution_evidence::Record, QualityError> {
    execution_evidence::passing_with_bindings(
        "fixture-mutation",
        format!(
            "{FIXTURE_CASES}#{}#{index}:{}",
            execution.case.id, execution.case.mutation
        ),
        FIXTURE_CASES,
        source.as_bytes(),
        tool_paths,
        tool_digest,
        configuration_paths,
        configuration_digest,
        format!(
            "mutation={};target-cardinality=1;polarity={};diagnostic={};count={}",
            execution.case.mutation, execution.case.polarity, execution.diagnostic, execution.count
        ),
    )
    .map_err(execution_evidence_error)
}

/// Render the sorted, deterministic execution-evidence text for a set of
/// already-executed fixture cases against their raw source rows.
///
/// Extracted from [`render_fixture_execution_evidence`] so the otherwise
/// unreachable defensive `missing source row` path (the parsed case list and
/// the raw manifest row extraction are always derived from the same
/// manifest text) and the final rendering failure path can be exercised
/// directly and honestly with a constructed input.
fn render_fixture_executions(
    executions: Vec<FixtureExecution>,
    source_rows: &BTreeMap<&str, &str>,
    tool_paths: &str,
    tool_digest: &str,
    configuration_paths: &str,
    configuration_digest: &str,
) -> Result<String, QualityError> {
    let mut records = Vec::new();
    for (index, execution) in executions.into_iter().enumerate() {
        if execution.case.mutation == "none" {
            continue;
        }
        let source = source_rows
            .get(execution.case.id.as_str())
            .ok_or_else(|| missing_source_row_error(&execution.case.id))?;
        records.push(fixture_execution_record(
            index,
            &execution,
            source,
            tool_paths,
            tool_digest,
            configuration_paths,
            configuration_digest,
        )?);
    }
    execution_evidence::render(&records).map_err(execution_evidence_error)
}

fn render_fixture_execution_evidence(root: &Path) -> Result<String, QualityError> {
    render_fixture_execution_evidence_bound_to(
        root,
        &EXECUTION_TOOL_PATHS,
        &EXECUTION_CONFIGURATION_PATHS,
    )
}

/// Core of [`render_fixture_execution_evidence`], parameterized over the
/// tool/configuration authority paths so the otherwise-unreachable binding
/// failure paths (the real constants always resolve against this repository
/// checkout's own real sources) can be exercised directly and honestly with
/// a deliberately invalid path, without disturbing any real repository file.
fn render_fixture_execution_evidence_bound_to(
    root: &Path,
    tool_authority_paths: &[&str],
    configuration_authority_paths: &[&str],
) -> Result<String, QualityError> {
    let base_text = read_text(root, FIXTURE_REGISTRY, "QA-FIXTURE-READ")?;
    let cases_text = read_text(root, FIXTURE_CASES, "QA-FIXTURE-READ")?;
    let mut base = parse_registry_text(&base_text, FIXTURE_REGISTRY)?;
    materialize_fixture_results(root, &mut base)?;
    let baseline = generate_from_registry(root, &base)?;
    let cases = parse_fixture_cases(&cases_text)?;
    let executions = execute_fixture_cases_with_results(root, &base_text, &base, &baseline, cases)?;
    let source_rows = cases_text
        .lines()
        .skip(1)
        .filter_map(|line| line.split_once('\t').map(|(_, tail)| (tail, line)))
        .filter_map(|(tail, line)| tail.split_once('\t').map(|(id, _)| (id, line)))
        .collect::<BTreeMap<_, _>>();
    let tool_paths = tool_authority_paths.join(";");
    let tool_digest = execution_tool_binding_digest(tool_authority_paths)?;
    let configuration_paths = configuration_authority_paths.join(";");
    let configuration_digest = execution_tool_binding_digest(configuration_authority_paths)?;
    render_fixture_executions(
        executions,
        &source_rows,
        &tool_paths,
        &tool_digest,
        &configuration_paths,
        &configuration_digest,
    )
}

/// Validate or rewrite the exact per-case quality execution artifact.
///
/// # Errors
///
/// Returns a stable diagnostic when a case fails or any source, tool,
/// configuration, or result binding is stale.
pub fn fixture_execution_evidence(root: &Path, write: bool) -> Result<(), QualityError> {
    let expected = render_fixture_execution_evidence(root)?;
    let path = root.join(FIXTURE_EXECUTIONS);
    execution_evidence::validate_materialized(&path.to_string_lossy(), &expected, write)
        .map_err(execution_evidence_error)
}

/// Execute every positive and negative registry fixture against exact diagnostics.
///
/// # Errors
///
/// Returns a stable diagnostic when fixture parsing, polarity, determinism, or cardinality fails.
pub fn self_test(root: &Path) -> Result<(), QualityError> {
    let base_text = read_text(root, FIXTURE_REGISTRY, "QA-FIXTURE-READ")?;
    let cases_text = read_text(root, FIXTURE_CASES, "QA-FIXTURE-READ")?;
    let mut base = parse_registry_text(&base_text, FIXTURE_REGISTRY)?;
    materialize_fixture_results(root, &mut base)?;
    let baseline = generate_from_registry(root, &base)?;
    let cases = parse_fixture_cases(&cases_text)?;
    execute_fixture_cases(root, &base_text, &base, &baseline, cases)?;
    coverage_controls(root, &base_text, &base, &baseline);
    execution_evidence_coverage_controls(root, &base);
    Ok(())
}

/// Exercise every read, parse, materialization, and generation failure that
/// [`render_fixture_execution_evidence`] can encounter, using an isolated
/// scratch root so the real committed fixtures are never disturbed.
fn execution_evidence_coverage_controls(root: &Path, base: &Registry) {
    let scratch = std::env::temp_dir().join(format!(
        "buoy-quality-execution-evidence-controls-{}",
        std::process::id()
    ));
    drop(fs::remove_dir_all(&scratch));
    fs::create_dir_all(&scratch)
        .expect("execution-evidence coverage-control scratch must be writable");

    let copy_in = |relative: &str, scratch: &Path| {
        let target = scratch.join(relative);
        fs::create_dir_all(
            target
                .parent()
                .expect("execution-evidence coverage-control copy target has a parent"),
        )
        .expect("execution-evidence coverage-control copy directory must be writable");
        fs::copy(root.join(relative), &target)
            .expect("execution-evidence coverage-control copy must succeed");
    };

    // Neither the frozen registry nor the case manifest exists yet; go
    // through the public checked/written wrapper so its own propagated
    // failure path is exercised too.
    expected_control_failure(fixture_execution_evidence(&scratch, false));

    copy_in(FIXTURE_REGISTRY, &scratch);
    // The registry exists but the case manifest does not.
    expected_control_failure(render_fixture_execution_evidence(&scratch));

    copy_in(FIXTURE_CASES, &scratch);
    fs::write(scratch.join(FIXTURE_REGISTRY), "[")
        .expect("execution-evidence coverage-control registry must be writable");
    // The registry text no longer parses.
    expected_control_failure(render_fixture_execution_evidence(&scratch));

    copy_in(FIXTURE_REGISTRY, &scratch);
    // The registry parses, but none of its result authorities exist yet.
    expected_control_failure(render_fixture_execution_evidence(&scratch));

    copy_authorities(root, &scratch, base)
        .expect("execution-evidence coverage-control authority copy must succeed");
    fs::remove_file(scratch.join(&base.gate_source))
        .expect("execution-evidence coverage-control gate source must be removable");
    // Every result authority is present, but the gate source is not.
    expected_control_failure(render_fixture_execution_evidence(&scratch));

    copy_in(&base.gate_source, &scratch);
    drop(render_fixture_execution_evidence(&scratch).expect(
        "execution-evidence coverage-control full scratch fixture must render successfully",
    ));

    fs::write(
        scratch.join(FIXTURE_CASES),
        "schema\tnot-the-frozen-header\n",
    )
    .expect("execution-evidence coverage-control case manifest must be writable");
    // The case manifest header is no longer the frozen shape.
    expected_control_failure(render_fixture_execution_evidence(&scratch));

    let cases_text = fs::read_to_string(root.join(FIXTURE_CASES))
        .expect("execution-evidence coverage-control case manifest must be readable");
    let mut duplicated = cases_text.clone();
    let first_row = cases_text
        .lines()
        .nth(1)
        .expect("execution-evidence coverage-control case manifest has a first row");
    duplicated.push_str(first_row);
    duplicated.push('\n');
    fs::write(scratch.join(FIXTURE_CASES), duplicated)
        .expect("execution-evidence coverage-control case manifest must be writable");
    // The case manifest now carries a duplicate case ID.
    expected_control_failure(render_fixture_execution_evidence(&scratch));

    fs::remove_dir_all(&scratch).expect("execution-evidence coverage-control cleanup must succeed");

    // The remaining failure paths below (a case ID the raw manifest text
    // cannot re-derive a source row for, and a tool/configuration authority
    // digest bound against a path that does not exist) cannot be produced by
    // mutating the on-disk fixture manifest, since the parsed case list and
    // the raw row extraction are always derived from the very same manifest
    // text. Exercise the private helpers directly here, alongside the
    // scratch-root controls above, so this repository-checkout invocation of
    // `self_test` (used by out-of-crate integration callers as well as the
    // in-crate unit tests) observes every failure path itself.
    let synthetic_missing = missing_source_row_error("QA-COVERAGE-CONTROL-SYNTHETIC");
    assert!(
        synthetic_missing
            .to_string()
            .contains("missing source row for QA-COVERAGE-CONTROL-SYNTHETIC"),
        "execution-evidence coverage-control missing-source-row diagnostic must name the case"
    );

    expected_control_failure(execution_tool_binding_digest(&[
        "/nonexistent/execution-evidence-quality-coverage-control",
    ]));

    expected_control_failure(render_fixture_execution_evidence_bound_to(
        root,
        &["/nonexistent/execution-evidence-quality-coverage-control-tool"],
        &EXECUTION_CONFIGURATION_PATHS,
    ));
    expected_control_failure(render_fixture_execution_evidence_bound_to(
        root,
        &EXECUTION_TOOL_PATHS,
        &["/nonexistent/execution-evidence-quality-coverage-control-configuration"],
    ));

    let synthetic_execution = FixtureExecution {
        case: FixtureCase {
            id: "QA-COVERAGE-CONTROL-SYNTHETIC".to_owned(),
            polarity: "negative".to_owned(),
            mutation: "synthetic-coverage-control".to_owned(),
            expected_diagnostic: "QA-SYNTHETIC".to_owned(),
            expected_count: 1,
        },
        diagnostic: "QA-SYNTHETIC".to_owned(),
        count: 1,
    };
    let empty_source_rows: BTreeMap<&str, &str> = BTreeMap::new();
    expected_control_failure(render_fixture_executions(
        vec![synthetic_execution.clone()],
        &empty_source_rows,
        "Cargo.toml",
        "sha-256:deadbeef",
        "Cargo.toml",
        "sha-256:deadbeef",
    ));
    expected_control_failure(fixture_execution_record(
        0,
        &synthetic_execution,
        "source line",
        "Cargo.toml",
        "",
        "Cargo.toml",
        "sha-256:deadbeef",
    ));
}

fn usage() -> QualityError {
    QualityError::from_diagnostic(Diagnostic::new(
        "QA-USAGE",
        "",
        "",
        "usage: buoy-quality (--self-test | --check | --generate | --check-execution-evidence | --write-execution-evidence) [--root ROOT] [--output DIR]",
    ))
}

enum CliAction {
    SelfTest,
    Check,
    Generate,
    CheckExecutionEvidence,
    WriteExecutionEvidence,
}

/// Run the stable `buoy-quality` command-line interface.
///
/// # Errors
///
/// Returns stable usage, validation, drift, or generation diagnostics.
pub fn run_cli(arguments: &[String], current_root: &Path) -> Result<(), QualityError> {
    let Some(first) = arguments.first() else {
        return Err(usage());
    };
    let action = match first.as_str() {
        "--self-test" => CliAction::SelfTest,
        "--check" => CliAction::Check,
        "--generate" => CliAction::Generate,
        "--check-execution-evidence" => CliAction::CheckExecutionEvidence,
        "--write-execution-evidence" => CliAction::WriteExecutionEvidence,
        _ => return Err(usage()),
    };
    let mut root = current_root.to_path_buf();
    let mut output: Option<PathBuf> = None;
    let mut index = 1;
    while index < arguments.len() {
        match arguments[index].as_str() {
            "--root" => {
                let Some(value) = arguments.get(index + 1) else {
                    return Err(usage());
                };
                root = PathBuf::from(value);
                index += 2;
            }
            "--output" => {
                let Some(value) = arguments.get(index + 1) else {
                    return Err(usage());
                };
                output = Some(PathBuf::from(value));
                index += 2;
            }
            _ => return Err(usage()),
        }
    }
    match action {
        CliAction::SelfTest => {
            if output.is_some() {
                Err(usage())
            } else {
                self_test(&root)
            }
        }
        CliAction::Check => {
            if output.is_some() {
                Err(usage())
            } else {
                check_repository(&root)
            }
        }
        CliAction::Generate => generate_repository(&root, output.as_deref()),
        CliAction::CheckExecutionEvidence => {
            if output.is_some() {
                Err(usage())
            } else {
                fixture_execution_evidence(&root, false)
            }
        }
        CliAction::WriteExecutionEvidence => {
            if output.is_some() {
                Err(usage())
            } else {
                fixture_execution_evidence(&root, true)
            }
        }
    }
}

#[cfg(test)]
mod execution_evidence_tests {
    use super::{
        FIXTURE_EXECUTIONS, FixtureCase, FixtureExecution, execution_evidence,
        execution_evidence_error, execution_tool_binding_digest, fixture_execution_record,
        missing_source_row_error, render_fixture_executions,
    };
    use std::collections::BTreeMap;

    #[test]
    fn execution_evidence_error_binds_the_stable_diagnostic_code_and_path() {
        let error = execution_evidence_error(String::from("synthetic failure"));
        let rendered = error.to_string();
        assert!(rendered.starts_with("QA-EXECUTION-EVIDENCE"));
        assert!(rendered.contains(FIXTURE_EXECUTIONS));
        assert!(rendered.contains("synthetic failure"));
    }

    #[test]
    fn missing_source_row_error_names_the_offending_case_id() {
        let error = missing_source_row_error("QA-FIX-SYNTHETIC");
        let rendered = error.to_string();
        assert!(rendered.starts_with("QA-EXECUTION-EVIDENCE"));
        assert!(rendered.contains("missing source row for QA-FIX-SYNTHETIC"));
    }

    #[test]
    fn execution_tool_binding_digest_binds_real_paths_and_fails_closed_for_missing_ones() {
        let digest = execution_tool_binding_digest(&["Cargo.toml"]).expect("real path must bind");
        assert!(buoy_core::SemanticDigest::parse_canonical(&digest).is_ok());
        let error = execution_tool_binding_digest(&[
            "/nonexistent/execution-evidence-quality-coverage-test",
        ])
        .expect_err("missing path must fail closed");
        assert!(error.to_string().starts_with("QA-EXECUTION-EVIDENCE"));
    }

    fn synthetic_execution(id: &str, mutation: &str) -> FixtureExecution {
        FixtureExecution {
            case: FixtureCase {
                id: id.to_owned(),
                polarity: "negative".to_owned(),
                mutation: mutation.to_owned(),
                expected_diagnostic: "QA-SYNTHETIC".to_owned(),
                expected_count: 1,
            },
            diagnostic: "QA-SYNTHETIC".to_owned(),
            count: 1,
        }
    }

    #[test]
    fn fixture_execution_record_binds_success_and_fails_closed_for_an_invalid_digest() {
        let execution = synthetic_execution("QA-FIX-SYNTHETIC", "synthetic-mutation");
        let record = fixture_execution_record(
            0,
            &execution,
            "source line",
            "Cargo.toml",
            "sha-256:deadbeef",
            "Cargo.toml",
            "sha-256:deadbeef",
        )
        .expect("well-formed inputs must bind a record");
        assert_eq!(record.category, "fixture-mutation");
        assert!(record.id.contains("QA-FIX-SYNTHETIC"));

        let error = fixture_execution_record(
            0,
            &execution,
            "source line",
            "Cargo.toml",
            "",
            "Cargo.toml",
            "sha-256:deadbeef",
        )
        .expect_err("an empty tool digest must fail closed");
        assert!(error.to_string().starts_with("QA-EXECUTION-EVIDENCE"));
    }

    #[test]
    fn render_fixture_executions_skips_none_mutation_and_fails_closed_without_a_source_row() {
        let empty_source_rows: BTreeMap<&str, &str> = BTreeMap::new();
        let rendered = render_fixture_executions(
            vec![synthetic_execution("QA-FIX-NONE", "none")],
            &empty_source_rows,
            "Cargo.toml",
            "sha-256:deadbeef",
            "Cargo.toml",
            "sha-256:deadbeef",
        )
        .expect("a `none` mutation is skipped and needs no source row");
        assert_eq!(rendered, execution_evidence::HEADER);

        let error = render_fixture_executions(
            vec![synthetic_execution("QA-FIX-MISSING", "synthetic-mutation")],
            &empty_source_rows,
            "Cargo.toml",
            "sha-256:deadbeef",
            "Cargo.toml",
            "sha-256:deadbeef",
        )
        .expect_err("a missing source row must fail closed");
        assert!(
            error
                .to_string()
                .contains("missing source row for QA-FIX-MISSING")
        );

        let mut present_source_rows: BTreeMap<&str, &str> = BTreeMap::new();
        present_source_rows.insert("QA-FIX-PRESENT", "source row text");
        let record_error = render_fixture_executions(
            vec![synthetic_execution("QA-FIX-PRESENT", "synthetic-mutation")],
            &present_source_rows,
            "Cargo.toml",
            "",
            "Cargo.toml",
            "sha-256:deadbeef",
        )
        .expect_err("an invalid tool digest must fail closed when building the record");
        assert!(
            record_error
                .to_string()
                .starts_with("QA-EXECUTION-EVIDENCE")
        );
    }

    #[test]
    fn render_fixture_execution_evidence_bound_to_binds_the_real_tree_and_fails_closed_for_invalid_authority_paths()
     {
        let root = std::path::Path::new(".");
        super::render_fixture_execution_evidence_bound_to(
            root,
            &super::EXECUTION_TOOL_PATHS,
            &super::EXECUTION_CONFIGURATION_PATHS,
        )
        .expect("the real repository tree binds successfully");

        let tool_error = super::render_fixture_execution_evidence_bound_to(
            root,
            &["/nonexistent/execution-evidence-quality-coverage-tool"],
            &super::EXECUTION_CONFIGURATION_PATHS,
        )
        .expect_err("a missing tool authority path must fail closed");
        assert!(tool_error.to_string().starts_with("QA-EXECUTION-EVIDENCE"));

        let configuration_error = super::render_fixture_execution_evidence_bound_to(
            root,
            &super::EXECUTION_TOOL_PATHS,
            &["/nonexistent/execution-evidence-quality-coverage-configuration"],
        )
        .expect_err("a missing configuration authority path must fail closed");
        assert!(
            configuration_error
                .to_string()
                .starts_with("QA-EXECUTION-EVIDENCE")
        );
    }
}
