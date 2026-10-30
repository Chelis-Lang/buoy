//! Materialized, fail-closed production-readiness obligation ledger.

#![allow(
    clippy::expect_used,
    clippy::format_push_string,
    clippy::items_after_statements,
    clippy::needless_pass_by_value,
    clippy::struct_excessive_bools,
    clippy::too_many_arguments,
    clippy::too_many_lines
)]

use super::content_digest;
use buoy::adversarial::{
    CampaignKind, platform_effect_diagnostics, portable_corpus, validate_repository_campaign,
};
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};
#[cfg(feature = "coverage-controls")]
use std::sync::atomic::{AtomicUsize, Ordering};

const LEDGER: &str = "assurance/v1/obligation-ledger.tsv";
const LEDGER_SCHEMA: &str = "assurance/v1/ledger-schema.toml";
const TOOL_SOURCE: &str = "tools/check_hardening_manifest.rs";
const LEDGER_SOURCE: &str = "tools/hardening_ledger.rs";
const CAMPAIGN_AS_OF: &str = "2026-10-30";
const HARDENING_SPECS: &str =
    "openspec/changes/archive/2026-08-02-harden-spec-provenance-production-readiness/specs";
const BASELINE_SPECS: &str = "openspec/specs";
const V2_MANIFEST: &str = "fixtures/v2/manifest.tsv";
const OBSERVATIONS: &str = "assurance/v1/fixture-behavioral-observations.tsv";
const SCHEMA_EXECUTIONS: &str = "assurance/v1/schema-fixture-executions.tsv";
const DEVENV_EXECUTIONS: &str = "assurance/v1/devenv-fixture-executions.tsv";
const DIGEST_EXECUTIONS: &str = "assurance/v1/digest-fixture-executions.tsv";
const PRODUCT_IDENTITY_EXECUTIONS: &str = "assurance/v1/product-identity-fixture-executions.tsv";
const QUALITY_EXECUTIONS: &str = "assurance/v1/quality-fixture-executions.tsv";
const REPOSITORY_EXECUTIONS: &str = "assurance/v1/repository-fixture-executions.tsv";
const ROLLBACK_EXECUTIONS: &str = "assurance/v1/product-identity-rollback-executions.tsv";
const MIGRATION_EXECUTIONS: &str = "assurance/v1/migration-fixture-executions.tsv";
const V2_EXECUTIONS: &str = "assurance/v1/v2-fixture-executions.tsv";
const MODEL_EXECUTIONS: &str = "assurance/v1/finite-model-production-conformance.tsv";
const FALSE_CONTROL_EXECUTION: &str = "assurance/v1/hegel-false-control-execution.tsv";
const SCENARIO_EXECUTIONS: &str = "assurance/v1/scenario-requirement-executions.tsv";
const EXECUTION_EVIDENCE_SCHEMA: &str = "buoy-local-execution-evidence/v1";
const SCHEMA_INVENTORY: &str = "schema/v3/inventory.tsv";
const SOURCE_INVENTORY: &str = "assurance/v1/source-inventory.tsv";
const COVERAGE_CONFIGURATION: &str = "assurance/v1/coverage-command.toml";
const COVERAGE_RESULT: &str = "assurance/v1/coverage-result.tsv";
const PROPERTY_INVENTORY: &str = "assurance/v1/property-inventory.tsv";
const PROPERTY_RUNS: &str = "assurance/v1/property-runs.tsv";
const NAMED_MUTANTS: &str = "assurance/adversarial/v1/named-mutants.tsv";
const MUTATION_ARTIFACT: &str = "assurance/adversarial/v1/mutation-campaign.toml";
const PLATFORM_BLOCKERS: &str = "assurance/v1/platform-blockers.toml";
const PLATFORM_RESULTS: &str = "assurance/adversarial/v1/platform-results";
#[cfg(target_os = "linux")]
const LOCAL_PLATFORM: &str = "linux";
#[cfg(target_os = "macos")]
const LOCAL_PLATFORM: &str = "macos";
#[cfg(not(any(target_os = "linux", target_os = "macos")))]
const LOCAL_PLATFORM: &str = "unsupported";
const PLATFORM_GENERATOR: &str = "tools/check_adversarial_evidence.rs";
const ADVERSARIAL_SOURCE: &str = "src/adversarial.rs";
const GATE_SOURCE: &str = "devenv.nix";
const PROOF_RECORD: &str = "proofs/core/proof-record.toml";
const DERIVATION_RECORD: &str = "proofs/core/derivation-record.toml";
const CORRESPONDENCE_APPROVAL: &str = "proofs/core/correspondence-approval.toml";
const CARGO_LOCK: &str = "Cargo.lock";
const DEVENV_LOCK: &str = "devenv.lock";
const SUPPLY_EXCEPTIONS: &str = "assurance/supply-chain/v1/exceptions.tsv";
const SUPPLY_SIZE_POLICY: &str = "assurance/supply-chain/v1/closure-size-policy.tsv";
const PHASE_HANDOFFS: &str = "assurance/v1/phase-handoffs.toml";
const NEGATIVE_DIGEST_ONLY: &str = "fixtures/assurance/v1/negative-digest-only-proof.tsv";
const LEGACY_SCHEMA_V2_APPROVAL: &str = "docs/schema/v3/README.md";
const LEDGER_SCHEMA_ID: &str = "spec-provenance-assurance-obligation/v1";
const HEADER: &str = "schema\tcategory\tid\trequirement_scope\tevidence_class\tdisposition\trationale\tsource_digest\ttool_digest\tconfiguration_digest\tevidence_paths\texpires_by_policy\tdownstream_exclusions\tblocker\tapproval\tresult_digest\tproperty\timplementation_representation\tcorrespondence\tproof_source\tverifier\tsolver\tconfiguration\tassumptions\tnegative_control\tformal_scope\tformal_exclusions\n";
const EXECUTION_HEADER: &str = "schema\tcategory\tid\tsource_paths\tsource_digest\ttool_paths\ttool_digest\tconfiguration_paths\tconfiguration_digest\tresult\tresult_digest\texecution\n";

#[cfg(feature = "coverage-controls")]
static COVERAGE_FAULT: AtomicUsize = AtomicUsize::new(0);

fn coverage_result<T>(site: usize, result: Result<T, String>) -> Result<T, String> {
    #[cfg(feature = "coverage-controls")]
    if COVERAGE_FAULT.load(Ordering::Relaxed) == site {
        return Err(format!("LEDGER-COVERAGE-CONTROL:{site}"));
    }
    let _ = site;
    result
}

const CATEGORIES: [&str; 13] = [
    "requirement",
    "scenario",
    "schema-row",
    "source-target",
    "fixture-operation",
    "fixture-mutation",
    "property",
    "finite-model",
    "mutation-class",
    "platform-backend",
    "proof-theorem",
    "supply-chain-input",
    "phase-handoff",
];

const EVIDENCE_CLASSES: [&str; 13] = [
    "point-fixture",
    "production-property-differential",
    "sampled-property",
    "production-finite-model-conformance",
    "structural-coverage",
    "structural-source-execution",
    "automated-mutation",
    "fuzz-campaign",
    "fault-injection",
    "platform-conformance",
    "formal-proof",
    "supply-chain-policy",
    "review-only",
];

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
struct Key {
    category: String,
    id: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct Entry {
    key: Key,
    requirement_scope: String,
    evidence_class: String,
    disposition: String,
    rationale: String,
    source_digest: String,
    tool_digest: Option<String>,
    configuration_digest: Option<String>,
    result_digest: String,
    evidence_paths: String,
    expires_by_policy: String,
    downstream_exclusions: String,
    blocker: String,
    approval: String,
    formal: Option<FormalFields>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct FormalFields {
    property: String,
    implementation_representation: String,
    correspondence: String,
    proof_source: String,
    verifier: String,
    solver: String,
    configuration: String,
    assumptions: String,
    negative_control: String,
    scope: String,
    exclusions: String,
}

#[derive(Clone, Debug)]
struct SpecRequirement {
    id: String,
    path: String,
    scenarios: Vec<String>,
}

#[derive(Clone, Debug)]
struct Table {
    header: Vec<String>,
    rows: Vec<Vec<String>>,
}

#[derive(Clone, Debug)]
struct ExecutionBinding {
    artifact: String,
    source_paths: String,
    source_digest: String,
    tool_paths: String,
    tool_digest: String,
    configuration_paths: String,
    configuration_digest: String,
    result_digest: String,
}

impl Table {
    fn column(&self, name: &str) -> Result<usize, String> {
        self.header
            .iter()
            .position(|field| field == name)
            .ok_or_else(|| format!("LEDGER-INVENTORY-SCHEMA: missing column `{name}`"))
    }
}

fn read_text(path: &str) -> Result<String, String> {
    fs::read_to_string(path).map_err(|error| format!("LEDGER-READ {path}: {error}"))
}

fn read_tsv(path: &str) -> Result<Table, String> {
    let text = read_text(path)?;
    let mut lines = text.lines();
    let header = lines
        .next()
        .ok_or_else(|| format!("LEDGER-INVENTORY-SCHEMA {path}: empty TSV"))?
        .split('\t')
        .map(str::to_owned)
        .collect::<Vec<_>>();
    let mut rows = Vec::new();
    for (index, line) in lines.enumerate() {
        let row = line.split('\t').map(str::to_owned).collect::<Vec<_>>();
        if row.len() != header.len() {
            return Err(format!(
                "LEDGER-INVENTORY-SCHEMA {path}: row {} has {}/{} fields",
                index + 2,
                row.len(),
                header.len()
            ));
        }
        rows.push(row);
    }
    Ok(Table { header, rows })
}

fn file_digest(path: &str) -> Result<String, String> {
    content_digest::file_binding(Path::new(path))
}

fn file_set_digest(paths: &[&str]) -> Result<String, String> {
    let mut bytes = Vec::new();
    for path in paths {
        let content = fs::read(path).map_err(|error| format!("LEDGER-READ {path}: {error}"))?;
        bytes.extend_from_slice(path.len().to_string().as_bytes());
        bytes.push(b':');
        bytes.extend_from_slice(path.as_bytes());
        bytes.extend_from_slice(content.len().to_string().as_bytes());
        bytes.push(b':');
        bytes.extend_from_slice(&content);
    }
    Ok(content_digest::bytes_binding(&bytes))
}

fn evidence_configuration_digest(paths: &str) -> Result<String, String> {
    let mut bound = vec![LEDGER_SCHEMA];
    bound.extend(paths.split(';'));
    file_set_digest(&bound)
}

fn row_digest(row: &[String]) -> String {
    content_digest::bytes_binding(row.join("\t").as_bytes())
}

fn path_list_digest(paths: &str) -> Result<String, String> {
    let values = paths.split(';').collect::<Vec<_>>();
    if values.iter().any(|path| path.is_empty()) {
        return Err(String::from(
            "EXECUTION-EVIDENCE-PATH: empty path in digest binding",
        ));
    }
    file_set_digest(&values)
}

fn cached_path_list_digest(
    paths: &str,
    cache: &mut BTreeMap<String, String>,
) -> Result<String, String> {
    if let Some(digest) = cache.get(paths) {
        return Ok(digest.clone());
    }
    let digest = path_list_digest(paths)?;
    cache.insert(paths.to_owned(), digest.clone());
    Ok(digest)
}

fn execution_bindings(paths: &[&str]) -> Result<BTreeMap<Key, ExecutionBinding>, String> {
    const EVIDENCE_HEADER: [&str; 12] = [
        "schema",
        "category",
        "id",
        "source_paths",
        "source_digest",
        "tool_paths",
        "tool_digest",
        "configuration_paths",
        "configuration_digest",
        "result",
        "result_digest",
        "execution",
    ];
    let mut bindings = BTreeMap::new();
    let mut source_tables = BTreeMap::<String, Table>::new();
    let mut path_digest_cache = BTreeMap::<String, String>::new();
    for path in paths {
        let table = read_tsv(path)?;
        if table.header.iter().map(String::as_str).collect::<Vec<_>>() != EVIDENCE_HEADER {
            return Err(format!(
                "EXECUTION-EVIDENCE-SCHEMA {path}: closed header differs"
            ));
        }
        for row in table.rows {
            let key = Key {
                category: row[1].clone(),
                id: row[2].clone(),
            };
            if row[0] != EXECUTION_EVIDENCE_SCHEMA
                || !matches!(
                    key.category.as_str(),
                    "fixture-operation"
                        | "fixture-mutation"
                        | "scenario"
                        | "requirement"
                        | "finite-model"
                        | "property"
                )
                || row[11] != "pass"
            {
                return Err(format!(
                    "EXECUTION-EVIDENCE-RESULT {path}: {}:{} is not an exact passing local execution",
                    key.category, key.id
                ));
            }
            if row[4]
                != content_digest::bytes_binding(
                    execution_digest_source(&key, &row[3], &mut source_tables)?.as_bytes(),
                )
            {
                return Err(format!(
                    "EXECUTION-EVIDENCE-SOURCE {path}: {}:{} source binding is stale",
                    key.category, key.id
                ));
            }
            if row[6] != cached_path_list_digest(&row[5], &mut path_digest_cache)?
                || row[8] != cached_path_list_digest(&row[7], &mut path_digest_cache)?
                || row[10] != content_digest::bytes_binding(row[9].as_bytes())
            {
                return Err(format!(
                    "EXECUTION-EVIDENCE-BINDING {path}: {}:{} tool, configuration, or result digest is stale",
                    key.category, key.id
                ));
            }
            let binding = ExecutionBinding {
                artifact: (*path).to_owned(),
                source_paths: row[3].clone(),
                source_digest: row[4].clone(),
                tool_paths: row[5].clone(),
                tool_digest: row[6].clone(),
                configuration_paths: row[7].clone(),
                configuration_digest: row[8].clone(),
                result_digest: row[10].clone(),
            };
            if bindings.insert(key.clone(), binding).is_some() {
                return Err(format!(
                    "EXECUTION-EVIDENCE-DUPLICATE {path}: {}:{}",
                    key.category, key.id
                ));
            }
        }
    }
    Ok(bindings)
}

fn execution_digest_source(
    key: &Key,
    source_paths: &str,
    source_tables: &mut BTreeMap<String, Table>,
) -> Result<String, String> {
    if source_paths.contains(';') {
        return Err(format!(
            "EXECUTION-EVIDENCE-SOURCE {}:{} must identify exactly one source",
            key.category, key.id
        ));
    }
    if Path::new(source_paths)
        .extension()
        .and_then(|value| value.to_str())
        != Some("tsv")
        || !key.id.contains('#')
    {
        return read_text(source_paths);
    }
    let (_, suffix) = key
        .id
        .split_once('#')
        .expect("the preceding guard already confirmed the id contains a source separator");
    if !source_tables.contains_key(source_paths) {
        source_tables.insert(source_paths.to_owned(), read_tsv(source_paths)?);
    }
    let table = source_tables
        .get(source_paths)
        .expect("the preceding branch already inserted this exact source table");
    let case = table
        .header
        .iter()
        .position(|field| matches!(field.as_str(), "case_id" | "case" | "id"))
        .ok_or_else(|| format!("EXECUTION-EVIDENCE-SOURCE {source_paths}: missing case column"))?;
    let case_id = suffix.split_once('#').map_or(suffix, |(value, _)| value);
    let row = table
        .rows
        .iter()
        .find(|row| row[case] == case_id)
        .ok_or_else(|| {
            format!("EXECUTION-EVIDENCE-SOURCE {source_paths}: missing case `{case_id}`")
        })?;
    Ok(row.join("\t"))
}

fn clean_field(value: &str, field: &str) -> Result<String, String> {
    if value.is_empty() || value.contains(['\t', '\r', '\n']) {
        Err(format!(
            "LEDGER-FIELD: {field} must be nonempty single-line UTF-8"
        ))
    } else {
        Ok(value.to_owned())
    }
}

fn blocked(
    category: &str,
    id: impl Into<String>,
    requirement_scope: impl Into<String>,
    evidence_class: &str,
    rationale: impl Into<String>,
    source_digest: String,
    evidence_paths: impl Into<String>,
    blocker: impl Into<String>,
    approval: impl Into<String>,
) -> Entry {
    Entry {
        key: Key {
            category: category.to_owned(),
            id: id.into(),
        },
        requirement_scope: requirement_scope.into(),
        evidence_class: evidence_class.to_owned(),
        disposition: "blocked".to_owned(),
        rationale: rationale.into(),
        source_digest,
        tool_digest: None,
        configuration_digest: None,
        result_digest: "-".to_owned(),
        evidence_paths: evidence_paths.into(),
        expires_by_policy: "none".to_owned(),
        downstream_exclusions: "no cross-class or aggregate-command substitution".to_owned(),
        blocker: blocker.into(),
        approval: approval.into(),
        formal: None,
    }
}

fn satisfied(
    category: &str,
    id: impl Into<String>,
    requirement_scope: impl Into<String>,
    evidence_class: &str,
    rationale: impl Into<String>,
    source_digest: String,
    evidence_paths: impl Into<String>,
) -> Entry {
    let result_digest = source_digest.clone();
    Entry {
        key: Key {
            category: category.to_owned(),
            id: id.into(),
        },
        requirement_scope: requirement_scope.into(),
        evidence_class: evidence_class.to_owned(),
        disposition: "satisfied".to_owned(),
        rationale: rationale.into(),
        source_digest,
        tool_digest: None,
        configuration_digest: None,
        result_digest,
        evidence_paths: evidence_paths.into(),
        expires_by_policy: "none".to_owned(),
        downstream_exclusions: "evidence is limited to the exact named class and scope".to_owned(),
        blocker: "-".to_owned(),
        approval: "-".to_owned(),
        formal: None,
    }
}

fn satisfied_execution(
    category: &str,
    id: impl Into<String>,
    requirement_scope: impl Into<String>,
    rationale: impl Into<String>,
    binding: &ExecutionBinding,
) -> Entry {
    let mut entry = satisfied(
        category,
        id,
        requirement_scope,
        "point-fixture",
        rationale,
        binding.source_digest.clone(),
        format!(
            "{};{};{};{}",
            binding.source_paths, binding.tool_paths, binding.configuration_paths, binding.artifact
        ),
    );
    entry.tool_digest = Some(binding.tool_digest.clone());
    entry.configuration_digest = Some(binding.configuration_digest.clone());
    entry.result_digest.clone_from(&binding.result_digest);
    entry
}

fn not_applicable(
    category: &str,
    id: impl Into<String>,
    requirement_scope: impl Into<String>,
    evidence_class: &str,
    rationale: impl Into<String>,
    source_digest: String,
    evidence_paths: impl Into<String>,
    approval: impl Into<String>,
) -> Entry {
    Entry {
        key: Key {
            category: category.to_owned(),
            id: id.into(),
        },
        requirement_scope: requirement_scope.into(),
        evidence_class: evidence_class.to_owned(),
        disposition: "not-applicable".to_owned(),
        rationale: rationale.into(),
        source_digest,
        tool_digest: None,
        configuration_digest: None,
        result_digest: "-".to_owned(),
        evidence_paths: evidence_paths.into(),
        expires_by_policy: "none".to_owned(),
        downstream_exclusions: "the scoped disposition grants no other evidence class".to_owned(),
        blocker: "-".to_owned(),
        approval: approval.into(),
        formal: None,
    }
}

fn insert(entries: &mut BTreeMap<Key, Entry>, entry: Entry) -> Result<(), String> {
    let key = entry.key.clone();
    if entries.insert(key.clone(), entry).is_some() {
        return Err(format!(
            "LEDGER-DUPLICATE: inventoried obligation identity {}:{}",
            key.category, key.id
        ));
    }
    Ok(())
}

fn directory_entry_path(
    entry: Result<fs::DirEntry, std::io::Error>,
    root: &Path,
) -> Result<PathBuf, String> {
    entry
        .map(|value| value.path())
        .map_err(|error| format!("LEDGER-READ {} entry: {error}", root.display()))
}

fn collect_files(
    root: &Path,
    file_name: Option<&str>,
    output: &mut Vec<PathBuf>,
) -> Result<(), String> {
    let entries =
        fs::read_dir(root).map_err(|error| format!("LEDGER-READ {}: {error}", root.display()))?;
    for entry in entries {
        let path = coverage_result(1, directory_entry_path(entry, root))?;
        if path.is_dir() {
            coverage_result(2, collect_files(&path, file_name, output))?;
        } else if file_name
            .is_none_or(|name| path.file_name().and_then(|part| part.to_str()) == Some(name))
        {
            output.push(path);
        }
    }
    Ok(())
}

fn read_spec_requirements() -> Result<Vec<SpecRequirement>, String> {
    let mut paths = Vec::new();
    coverage_result(
        3,
        collect_files(Path::new(BASELINE_SPECS), Some("spec.md"), &mut paths),
    )?;
    coverage_result(
        4,
        fs::metadata(HARDENING_SPECS)
            .map(|_| ())
            .map_err(|error| format!("LEDGER-READ {HARDENING_SPECS}: {error}")),
    )?;
    paths.sort();
    let mut requirements = Vec::new();
    for path in paths {
        let relative = path.to_string_lossy().into_owned();
        let text = coverage_result(5, read_text(&relative))?;
        let mut current: Option<usize> = None;
        for line in text.lines() {
            if let Some(heading) = line.strip_prefix("### Requirement: ") {
                let id = heading
                    .split_whitespace()
                    .next()
                    .ok_or_else(|| format!("LEDGER-SPEC {relative}: empty requirement"))?;
                requirements.push(SpecRequirement {
                    id: id.to_owned(),
                    path: relative.clone(),
                    scenarios: Vec::new(),
                });
                current = Some(requirements.len() - 1);
            } else if let Some(heading) = line.strip_prefix("#### Scenario: ") {
                let index = current.ok_or_else(|| {
                    format!("LEDGER-SPEC {relative}: scenario precedes requirement")
                })?;
                let title = heading
                    .strip_prefix("Positive - ")
                    .or_else(|| heading.strip_prefix("Negative - "))
                    .ok_or_else(|| format!("LEDGER-SPEC {relative}: scenario lacks polarity"))?;
                requirements[index].scenarios.push(slugify(title));
            }
        }
    }
    let mut seen = BTreeSet::new();
    for requirement in &requirements {
        if !seen.insert(requirement.id.clone()) || requirement.scenarios.is_empty() {
            return Err(format!(
                "LEDGER-SPEC: duplicate or scenarioless requirement {}",
                requirement.id
            ));
        }
    }
    Ok(requirements)
}

fn slugify(input: &str) -> String {
    let mut result = String::new();
    let mut separator = false;
    for character in input.chars().flat_map(char::to_lowercase) {
        if character.is_alphanumeric() {
            if separator && !result.is_empty() {
                result.push('-');
            }
            result.push(character);
            separator = false;
        } else {
            separator = true;
        }
    }
    result
}

fn observed_fixture_cases() -> Result<BTreeSet<String>, String> {
    let table = read_tsv(OBSERVATIONS)?;
    let case = table.column("case_id")?;
    Ok(table
        .rows
        .into_iter()
        .map(|row| row[case].clone())
        .collect())
}

fn v2_scenario_cases() -> Result<BTreeMap<(String, String), String>, String> {
    let table = read_tsv(V2_MANIFEST)?;
    let requirement = table.column("requirement")?;
    let scenario = table.column("spec_scenario")?;
    let case = table.column("case_id")?;
    let mut result = BTreeMap::new();
    for row in table.rows {
        let scenario = row[scenario]
            .rsplit_once('/')
            .map_or(row[scenario].as_str(), |(_, value)| value)
            .to_owned();
        result.insert((row[requirement].clone(), scenario), row[case].clone());
    }
    Ok(result)
}

fn add_requirements(entries: &mut BTreeMap<Key, Entry>) -> Result<(), String> {
    let executions = execution_bindings(&[SCENARIO_EXECUTIONS])?;
    for requirement in read_spec_requirements()? {
        let digest = coverage_result(6, file_digest(&requirement.path))?;
        for scenario in &requirement.scenarios {
            let id = format!("{}:{scenario}", requirement.id);
            let key = Key {
                category: String::from("scenario"),
                id: id.clone(),
            };
            let entry = if let Some(binding) = executions.get(&key) {
                satisfied_execution(
                    "scenario",
                    id,
                    format!("{} scenario {scenario}", requirement.id),
                    "the scenario produced its own normalized source/tool/configuration/result-bound execution record",
                    binding,
                )
            } else {
                blocked(
                    "scenario",
                    id,
                    format!("{} scenario {scenario}", requirement.id),
                    "point-fixture",
                    "no exact current normalized scenario execution artifact is inventoried",
                    digest.clone(),
                    &requirement.path,
                    SCENARIO_EXECUTIONS,
                    LEDGER_SCHEMA,
                )
            };
            insert(entries, entry)?;
        }
        let key = Key {
            category: String::from("requirement"),
            id: requirement.id.clone(),
        };
        let entry = if let Some(binding) = executions.get(&key) {
            satisfied_execution(
                "requirement",
                &requirement.id,
                format!("complete requirement {}", requirement.id),
                "the requirement produced its own normalized result bound to every separate scenario result digest",
                binding,
            )
        } else {
            blocked(
                "requirement",
                &requirement.id,
                format!("complete requirement {}", requirement.id),
                "point-fixture",
                "one or more frozen scenarios lack exact current normalized execution evidence",
                digest,
                &requirement.path,
                SCENARIO_EXECUTIONS,
                LEDGER_SCHEMA,
            )
        };
        insert(entries, entry)?;
    }
    if executions.keys().any(|key| !entries.contains_key(key)) {
        return Err(String::from(
            "EXECUTION-EVIDENCE-ORPHAN: scenario artifact contains a non-inventoried scenario or requirement",
        ));
    }
    Ok(())
}

fn add_schema_rows(entries: &mut BTreeMap<Key, Entry>) -> Result<(), String> {
    let table = read_tsv(SCHEMA_INVENTORY)?;
    let category = table.column("category")?;
    let parent = table.column("parent")?;
    let name = table.column("name")?;
    let tag = table.column("tag")?;
    for row in table.rows {
        let id = format!(
            "{}:{}:{}:{}",
            row[category], row[parent], row[name], row[tag]
        );
        insert(
            entries,
            satisfied(
                "schema-row",
                id,
                "one exact frozen schema inventory row",
                "point-fixture",
                "the row is selected by the strict schema parity authority",
                row_digest(&row),
                format!("{SCHEMA_INVENTORY};tools/check_schema_inventory.rs"),
            ),
        )?;
    }
    Ok(())
}

fn coverage_rows() -> Result<BTreeMap<String, Vec<String>>, String> {
    let table = read_tsv(COVERAGE_RESULT)?;
    let id = table.column("target_id")?;
    Ok(table
        .rows
        .into_iter()
        .map(|row| (row[id].clone(), row))
        .collect())
}

fn exact_coverage(row: &[String], table: &Table, source_digest: &str) -> Result<bool, String> {
    for name in ["lines", "regions", "functions", "branches"] {
        let covered = table.column(&format!("{name}_covered"))?;
        let count = table.column(&format!("{name}_count"))?;
        if row[covered] != row[count] {
            return Ok(false);
        }
    }
    Ok(row[table.column("source_digest")?] == source_digest
        && row[table.column("registration")?] == "present"
        && row[table.column("freshness")?] == "current"
        && row[table.column("execution")?] == "pass"
        && row[table.column("assurance_class")?] == "structural-source-execution")
}

fn add_source_targets(entries: &mut BTreeMap<Key, Entry>) -> Result<(), String> {
    let inventory = read_tsv(SOURCE_INVENTORY)?;
    let id = inventory.column("target_id")?;
    let path = inventory.column("path")?;
    let class = inventory.column("coverage_class")?;
    let disposition = inventory.column("disposition")?;
    let rationale = inventory.column("rationale")?;
    let requires_recorded_coverage = inventory.rows.iter().any(|row| {
        !matches!(
            row[disposition].as_str(),
            "not-applicable"
                | "included-generated"
                | "included-manual"
                | "included-assurance-manual"
                | "included-fixture"
        ) && !row[disposition].starts_with("excluded-")
    });
    let recorded_coverage = if requires_recorded_coverage {
        Some((read_tsv(COVERAGE_RESULT)?, coverage_rows()?))
    } else {
        None
    };
    for row in inventory.rows {
        let source_digest = file_digest(&row[path])?;
        let disposition_value = row[disposition].as_str();
        let entry = if disposition_value == "not-applicable"
            || disposition_value.starts_with("excluded-")
            || disposition_value == "included-generated"
        {
            not_applicable(
                "source-target",
                &row[id],
                format!("{} ({})", row[path], row[class]),
                "structural-source-execution",
                &row[rationale],
                source_digest,
                format!("{SOURCE_INVENTORY};{}", row[path]),
                SOURCE_INVENTORY,
            )
        } else if matches!(
            disposition_value,
            "included-manual" | "included-assurance-manual"
        ) {
            not_applicable(
                "source-target",
                &row[id],
                format!("{} ({})", row[path], row[class]),
                "structural-source-execution",
                "the source remains in the explicit coverage task and is outside automatic acceptance",
                source_digest,
                format!("{SOURCE_INVENTORY};{}", row[path]),
                SOURCE_INVENTORY,
            )
        } else if disposition_value == "included-fixture" {
            satisfied(
                "source-target",
                &row[id],
                format!("{} ({})", row[path], row[class]),
                "point-fixture",
                "the source is an executed fixture authority, not a product coverage target",
                source_digest,
                format!("{SOURCE_INVENTORY};{}", row[path]),
            )
        } else if recorded_coverage.as_ref().is_some_and(|(table, results)| {
            results.get(&row[id]).is_some_and(|result| {
                exact_coverage(result, table, &source_digest).unwrap_or(false)
            })
        }) {
            satisfied(
                "source-target",
                &row[id],
                format!("{} ({})", row[path], row[class]),
                "structural-source-execution",
                "all four exact integer metrics are current and equal for this source target",
                source_digest,
                format!("{SOURCE_INVENTORY};{COVERAGE_RESULT};{}", row[path]),
            )
        } else {
            blocked(
                "source-target",
                &row[id],
                format!("{} ({})", row[path], row[class]),
                "structural-source-execution",
                "the source target is absent, stale, or not exact in the current coverage result",
                source_digest,
                format!("{SOURCE_INVENTORY};{COVERAGE_RESULT};{}", row[path]),
                COVERAGE_RESULT,
                SOURCE_INVENTORY,
            )
        };
        insert(entries, entry)?;
    }
    Ok(())
}

fn add_v2_fixture_entries(entries: &mut BTreeMap<Key, Entry>) -> Result<(), String> {
    let manifest = read_tsv(V2_MANIFEST)?;
    let fixture = manifest.column("fixture")?;
    let case_id = manifest.column("case_id")?;
    let executions = execution_bindings(&[V2_EXECUTIONS])?;
    let mut paths = BTreeSet::new();
    for row in &manifest.rows {
        paths.insert(row[fixture].clone());
    }
    for path in paths {
        let text = read_text(&path)?;
        let value = toml::from_str::<toml::Value>(&text)
            .map_err(|_| format!("LEDGER-FIXTURE-SCHEMA {path}: invalid TOML"))?;
        let cases = value
            .get("case")
            .and_then(toml::Value::as_array)
            .ok_or_else(|| format!("LEDGER-FIXTURE-SCHEMA {path}: missing cases"))?;
        for case in cases {
            let id = case
                .get("id")
                .and_then(toml::Value::as_str)
                .ok_or_else(|| format!("LEDGER-FIXTURE-SCHEMA {path}: missing case ID"))?;
            let operation = case
                .get("operation")
                .and_then(toml::Value::as_str)
                .ok_or_else(|| format!("LEDGER-FIXTURE-SCHEMA {path}: missing operation"))?;
            let digest = coverage_result(7, file_digest(&path))?;
            let operation_id = format!("{path}#{id}#{operation}");
            let operation_key = Key {
                category: String::from("fixture-operation"),
                id: operation_id.clone(),
            };
            let operation_entry = if let Some(binding) = executions.get(&operation_key) {
                satisfied_execution(
                    "fixture-operation",
                    operation_id,
                    format!("case {id} operation {operation}"),
                    "the exact operation produced its own production/reference result record",
                    binding,
                )
            } else {
                blocked(
                    "fixture-operation",
                    operation_id,
                    format!("case {id} operation {operation}"),
                    "point-fixture",
                    "the operation has no separate source/tool/configuration/result-bound execution record",
                    digest.clone(),
                    &path,
                    V2_EXECUTIONS,
                    LEDGER_SCHEMA,
                )
            };
            insert(entries, operation_entry)?;
            let mutations = case
                .get("mutations")
                .and_then(toml::Value::as_array)
                .ok_or_else(|| format!("LEDGER-FIXTURE-SCHEMA {path}: missing mutations"))?;
            for (index, mutation) in mutations.iter().enumerate() {
                let mutation = mutation
                    .as_str()
                    .ok_or_else(|| format!("LEDGER-FIXTURE-SCHEMA {path}: non-string mutation"))?;
                let mutation_id = format!("{path}#{id}#{index}:{mutation}");
                let mutation_key = Key {
                    category: String::from("fixture-mutation"),
                    id: mutation_id.clone(),
                };
                let entry = if let Some(binding) = executions.get(&mutation_key) {
                    satisfied_execution(
                        "fixture-mutation",
                        mutation_id,
                        format!("case {id} ordered mutation {index}: {mutation}"),
                        "the exact ordered mutation produced its own production/reference witness",
                        binding,
                    )
                } else {
                    blocked(
                        "fixture-mutation",
                        mutation_id,
                        format!("case {id} ordered mutation {index}: {mutation}"),
                        "point-fixture",
                        "the mutation has no separate target-cardinality and result-bound witness",
                        digest.clone(),
                        &path,
                        V2_EXECUTIONS,
                        LEDGER_SCHEMA,
                    )
                };
                insert(entries, entry)?;
            }
        }
    }
    let manifest_cases = manifest
        .rows
        .iter()
        .map(|row| row[case_id].as_str())
        .collect::<BTreeSet<_>>();
    if manifest_cases.len() != manifest.rows.len() {
        return Err(String::from("LEDGER-FIXTURE-SCHEMA: duplicate v2 case ID"));
    }
    if executions.keys().any(|key| !entries.contains_key(key)) {
        return Err(String::from(
            "EXECUTION-EVIDENCE-ORPHAN: v2 execution artifact contains a non-inventoried operation or mutation",
        ));
    }
    Ok(())
}

fn legacy_schema_v2_fixture(path: &str) -> bool {
    matches!(
        path,
        "fixtures/hardening/v2/schema-construction-fixtures.tsv"
            | "fixtures/hardening/v2/schema-state-fixtures.tsv"
            | "fixtures/hardening/v2/impact-edge-fixtures.tsv"
            | "fixtures/hardening/v2/policy-ratchet-fixtures.tsv"
            | "fixtures/hardening/v2/disposition-protocol-fixtures.tsv"
    )
}

fn add_tsv_fixture_entries(entries: &mut BTreeMap<Key, Entry>) -> Result<(), String> {
    let executions = execution_bindings(&[
        SCHEMA_EXECUTIONS,
        DEVENV_EXECUTIONS,
        DIGEST_EXECUTIONS,
        PRODUCT_IDENTITY_EXECUTIONS,
        QUALITY_EXECUTIONS,
        REPOSITORY_EXECUTIONS,
        ROLLBACK_EXECUTIONS,
        MIGRATION_EXECUTIONS,
    ])?;
    let mut paths = Vec::new();
    collect_files(Path::new("fixtures"), None, &mut paths)?;
    paths.sort();
    for path in paths {
        if path.extension().and_then(|value| value.to_str()) != Some("tsv") {
            continue;
        }
        let relative = path.to_string_lossy().into_owned();
        if relative == V2_MANIFEST || relative == NEGATIVE_DIGEST_ONLY {
            continue;
        }
        let table = read_tsv(&relative)?;
        let case = table
            .header
            .iter()
            .position(|field| matches!(field.as_str(), "case_id" | "case" | "id"));
        let operation = table.header.iter().position(|field| field == "operation");
        let mutation = table.header.iter().position(|field| field == "mutation");
        let Some(case) = case else { continue };
        for (index, row) in table.rows.iter().enumerate() {
            if let Some(operation) = operation {
                let id = format!("{relative}#{}#{}", row[case], row[operation]);
                let key = Key {
                    category: "fixture-operation".to_owned(),
                    id: id.clone(),
                };
                let entry = if let Some(binding) = executions.get(&key) {
                    satisfied_execution(
                        "fixture-operation",
                        id,
                        format!("{} operation {}", row[case], row[operation]),
                        "the exact operation produced its own source/tool/configuration/result-bound execution record",
                        binding,
                    )
                } else if legacy_schema_v2_fixture(&relative) {
                    not_applicable(
                        "fixture-operation",
                        id,
                        format!("{} operation {}", row[case], row[operation]),
                        "point-fixture",
                        "the schema-v2 operation is a historical cutover fixture with no schema-v3 execution claim",
                        row_digest(row),
                        format!("{relative};{LEGACY_SCHEMA_V2_APPROVAL}"),
                        LEGACY_SCHEMA_V2_APPROVAL,
                    )
                } else {
                    blocked(
                        "fixture-operation",
                        id,
                        format!("{} operation {}", row[case], row[operation]),
                        "point-fixture",
                        "the fixture operation is inventoried but has no separate normalized per-case result artifact",
                        row_digest(row),
                        &relative,
                        &relative,
                        LEDGER_SCHEMA,
                    )
                };
                insert(entries, entry)?;
            }
            if let Some(mutation) = mutation {
                let value = row[mutation].trim();
                if !value.is_empty() && !matches!(value, "none" | "-") {
                    let id = format!("{relative}#{}#{index}:{value}", row[case]);
                    let key = Key {
                        category: "fixture-mutation".to_owned(),
                        id: id.clone(),
                    };
                    let entry = if let Some(binding) = executions.get(&key) {
                        satisfied_execution(
                            "fixture-mutation",
                            id,
                            format!("{} mutation {value}", row[case]),
                            "the exact mutation produced its own target-cardinality and source/tool/configuration/result-bound execution record",
                            binding,
                        )
                    } else if legacy_schema_v2_fixture(&relative) {
                        not_applicable(
                            "fixture-mutation",
                            id,
                            format!("{} mutation {value}", row[case]),
                            "point-fixture",
                            "the schema-v2 mutation is a historical cutover fixture with no schema-v3 execution claim",
                            row_digest(row),
                            format!("{relative};{LEGACY_SCHEMA_V2_APPROVAL}"),
                            LEGACY_SCHEMA_V2_APPROVAL,
                        )
                    } else {
                        blocked(
                            "fixture-mutation",
                            id,
                            format!("{} mutation {value}", row[case]),
                            "point-fixture",
                            "the fixture mutation is inventoried but has no separate normalized per-case result artifact",
                            row_digest(row),
                            &relative,
                            &relative,
                            LEDGER_SCHEMA,
                        )
                    };
                    insert(entries, entry)?;
                }
            }
        }
    }
    if executions.keys().any(|key| !entries.contains_key(key)) {
        return Err(String::from(
            "EXECUTION-EVIDENCE-ORPHAN: fixture execution artifact contains a non-inventoried operation or mutation",
        ));
    }
    Ok(())
}

fn add_properties(entries: &mut BTreeMap<Key, Entry>) -> Result<(), String> {
    let inventory = read_tsv(PROPERTY_INVENTORY)?;
    let runs = read_tsv(PROPERTY_RUNS)?;
    let executions = execution_bindings(&[FALSE_CONTROL_EXECUTION])?;
    let inventory_id = inventory.column("property_id")?;
    let inventory_status = inventory.column("status")?;
    let inventory_scope = inventory.column("scope")?;
    let run_id = runs.column("property_id")?;
    let execution = runs.column("execution")?;
    let registration = runs.column("registration")?;
    let freshness = runs.column("freshness")?;
    let source_digest = runs.column("production_source_digest")?;
    let mut current = BTreeMap::new();
    for row in runs.rows {
        current.insert(row[run_id].clone(), row);
    }
    for row in inventory.rows {
        let run = current.get(&row[inventory_id]);
        let passing = run.is_some_and(|value| {
            matches!(value[execution].as_str(), "pass" | "fail")
                && value[registration] == "present"
                && value[freshness] == "current"
        });
        let class = if row[inventory_status] == "production-differential" {
            "production-property-differential"
        } else {
            "sampled-property"
        };
        let digest = run.map_or_else(
            || row_digest(&row),
            |value| {
                if value[source_digest] == "not-applicable" {
                    row_digest(value)
                } else {
                    value[source_digest].clone()
                }
            },
        );
        let key = Key {
            category: String::from("property"),
            id: row[inventory_id].clone(),
        };
        let entry = if let Some(binding) = executions.get(&key) {
            let mut entry = satisfied_execution(
                "property",
                &row[inventory_id],
                &row[inventory_scope],
                "the intentionally-false property produced its own exact expected-failure result",
                binding,
            );
            class.clone_into(&mut entry.evidence_class);
            entry
        } else if passing {
            satisfied(
                "property",
                &row[inventory_id],
                &row[inventory_scope],
                class,
                "the exact source/tool/settings/domain-bound property result is current",
                digest,
                format!("{PROPERTY_INVENTORY};{PROPERTY_RUNS}"),
            )
        } else {
            blocked(
                "property",
                &row[inventory_id],
                &row[inventory_scope],
                class,
                "the property has no exact current registered result",
                digest,
                format!("{PROPERTY_INVENTORY};{PROPERTY_RUNS}"),
                PROPERTY_RUNS,
                PROPERTY_INVENTORY,
            )
        };
        insert(entries, entry)?;
    }
    if executions.keys().any(|key| !entries.contains_key(key)) {
        return Err(String::from(
            "EXECUTION-EVIDENCE-ORPHAN: false-control artifact contains a non-inventoried property",
        ));
    }
    Ok(())
}

fn add_models(entries: &mut BTreeMap<Key, Entry>) -> Result<(), String> {
    let executions = execution_bindings(&[MODEL_EXECUTIONS])?;
    let mut paths = Vec::new();
    for root in ["fixtures/v2/models", "fixtures/hardening/v2/models"] {
        collect_files(Path::new(root), None, &mut paths)?;
    }
    paths.sort();
    for path in paths {
        if path.extension().and_then(|value| value.to_str()) != Some("toml") {
            continue;
        }
        let relative = path.to_string_lossy().into_owned();
        let id = relative
            .strip_prefix("fixtures/")
            .unwrap_or(&relative)
            .trim_end_matches(".toml")
            .replace('/', ":");
        let key = Key {
            category: String::from("finite-model"),
            id: id.clone(),
        };
        let digest = coverage_result(8, file_digest(&relative))?;
        let entry = if let Some(binding) = executions.get(&key) {
            let mut entry = satisfied_execution(
                "finite-model",
                id,
                format!("every reachable state and transition in {relative}"),
                "the exact bounded model produced its own exhaustive production/reference conformance result",
                binding,
            );
            entry.evidence_class = String::from("production-finite-model-conformance");
            entry
        } else {
            blocked(
                "finite-model",
                id,
                format!("every reachable state and transition in {relative}"),
                "production-finite-model-conformance",
                "the model is executable but has no separately materialized current production-conformance result",
                digest,
                format!("{relative};tools/check_fixture_manifest.rs"),
                MODEL_EXECUTIONS,
                LEDGER_SCHEMA,
            )
        };
        insert(entries, entry)?;
    }
    if executions.keys().any(|key| !entries.contains_key(key)) {
        return Err(String::from(
            "EXECUTION-EVIDENCE-ORPHAN: finite-model artifact contains a non-inventoried model",
        ));
    }
    Ok(())
}

fn campaign_is_current(kind: CampaignKind) -> bool {
    validate_repository_campaign(Path::new("."), kind, CAMPAIGN_AS_OF).is_ok()
}

fn add_mutations_with_current(
    entries: &mut BTreeMap<Key, Entry>,
    current: bool,
) -> Result<(), String> {
    let table = read_tsv(NAMED_MUTANTS)?;
    let id = table.column("id")?;
    let category = table.column("category")?;
    let source = table.column("source")?;
    for row in table.rows {
        let mut entry = if current {
            satisfied(
                "mutation-class",
                &row[id],
                format!("{} mutation at {}", row[category], row[source]),
                "automated-mutation",
                "the named tripwire and current accepted mutation campaign unit are both present",
                file_digest(&row[source])?,
                format!("{NAMED_MUTANTS};{MUTATION_ARTIFACT};{}", row[source]),
            )
        } else {
            blocked(
                "mutation-class",
                &row[id],
                format!("{} mutation at {}", row[category], row[source]),
                "automated-mutation",
                "the current mutation campaign artifact is absent or failing",
                file_digest(&row[source])?,
                format!("{NAMED_MUTANTS};{MUTATION_ARTIFACT};{}", row[source]),
                MUTATION_ARTIFACT,
                NAMED_MUTANTS,
            )
        };
        "assurance/adversarial/v1/mutation-policy.toml#maximum_age_days"
            .clone_into(&mut entry.expires_by_policy);
        insert(entries, entry)?;
    }
    Ok(())
}

fn add_mutations(entries: &mut BTreeMap<Key, Entry>) -> Result<(), String> {
    add_mutations_with_current(entries, campaign_is_current(CampaignKind::Mutation))
}

fn platform_result_is_current_at(root: &Path, platform: &str) -> bool {
    let directory = root.join(platform);
    let result_path = directory.join("result.toml");
    let Ok(result_text) = fs::read_to_string(result_path) else {
        return false;
    };
    let Ok(result) = toml::from_str::<toml::Value>(&result_text) else {
        return false;
    };
    if result.get("schema").and_then(toml::Value::as_str) != Some("buoy-platform-result/v1")
        || result.get("platform").and_then(toml::Value::as_str) != Some(platform)
        || result.get("command").and_then(toml::Value::as_str)
            != Some("devenv shell -- buoy-platform-result")
        || result.get("status").and_then(toml::Value::as_str) != Some("green")
        || result.get("provenance").and_then(toml::Value::as_str) != Some("local-host-execution")
        || result.get("portable_corpus").and_then(toml::Value::as_str)
            != Some("portable-corpus.tsv")
        || result.get("platform_effects").and_then(toml::Value::as_str)
            != Some("platform-effects.tsv")
    {
        return false;
    }
    let corpus_matches = fs::read_to_string(directory.join("portable-corpus.tsv"))
        .ok()
        .zip(portable_corpus().ok())
        .is_some_and(|(actual, expected)| actual == expected);
    let effects_match = fs::read_to_string(directory.join("platform-effects.tsv"))
        .ok()
        .zip(platform_effect_diagnostics(platform).ok())
        .is_some_and(|(actual, expected)| actual == expected);
    corpus_matches && effects_match
}

fn platform_result_is_current(platform: &str) -> bool {
    platform_result_is_current_at(Path::new(PLATFORM_RESULTS), platform)
}

fn add_platforms(entries: &mut BTreeMap<Key, Entry>) -> Result<(), String> {
    for (platform, id, scope) in [
        (
            "linux",
            "linux-portable-corpus",
            "Linux canonical corpus and effect diagnostics",
        ),
        (
            "macos",
            "macos-portable-corpus",
            "macOS canonical corpus and effect diagnostics",
        ),
    ] {
        let result_directory = format!("{PLATFORM_RESULTS}/{platform}");
        let entry = if platform == LOCAL_PLATFORM && platform_result_is_current(platform) {
            satisfied(
                "platform-backend",
                id,
                scope,
                "platform-conformance",
                "the current-host local result exactly matches the canonical corpus and scoped effect diagnostics",
                file_set_digest(&[ADVERSARIAL_SOURCE, PLATFORM_GENERATOR])?,
                format!(
                    "{GATE_SOURCE};{ADVERSARIAL_SOURCE};{PLATFORM_GENERATOR};{result_directory}/result.toml;{result_directory}/portable-corpus.tsv;{result_directory}/platform-effects.tsv"
                ),
            )
        } else if platform != LOCAL_PLATFORM {
            not_applicable(
                "platform-backend",
                id,
                scope,
                "platform-conformance",
                "the local-only acceptance policy does not claim conformance for a noncurrent host",
                file_digest(PHASE_HANDOFFS)?,
                PHASE_HANDOFFS,
                PHASE_HANDOFFS,
            )
        } else {
            blocked(
                "platform-backend",
                id,
                scope,
                "platform-conformance",
                "the current host has no normalized result that matches the canonical corpus and scoped effect diagnostics",
                file_digest(PHASE_HANDOFFS)?,
                format!("{PHASE_HANDOFFS};{GATE_SOURCE}"),
                PHASE_HANDOFFS,
                PHASE_HANDOFFS,
            )
        };
        insert(entries, entry)?;
    }
    for (id, scope) in [
        (
            "PLATFORM-WINDOWS-SNAPSHOT-001",
            "Windows handle-relative snapshot backend",
        ),
        (
            "PLATFORM-WINDOWS-JOB-001",
            "Windows process job-object backend",
        ),
    ] {
        insert(
            entries,
            not_applicable(
                "platform-backend",
                id,
                scope,
                "platform-conformance",
                "windows is outside the reviewed supported-platform scope; production code fails closed and claims no windows backend",
                file_digest(PLATFORM_BLOCKERS)?,
                PLATFORM_BLOCKERS,
                PLATFORM_BLOCKERS,
            ),
        )?;
    }
    Ok(())
}

fn required_string<'a>(value: &'a toml::Value, path: &[&str]) -> Result<&'a str, String> {
    let mut current = value;
    for component in path {
        current = current
            .get(*component)
            .ok_or_else(|| format!("LEDGER-PROOF-BINDING: missing {}", path.join(".")))?;
    }
    current
        .as_str()
        .ok_or_else(|| format!("LEDGER-PROOF-BINDING: non-string {}", path.join(".")))
}

fn string_array(value: &toml::Value, path: &[&str]) -> Result<Vec<String>, String> {
    let mut current = value;
    for component in path {
        current = current
            .get(*component)
            .ok_or_else(|| format!("LEDGER-PROOF-BINDING: missing {}", path.join(".")))?;
    }
    current
        .as_array()
        .ok_or_else(|| format!("LEDGER-PROOF-BINDING: non-array {}", path.join(".")))?
        .iter()
        .map(|item| {
            item.as_str()
                .map(str::to_owned)
                .ok_or_else(|| format!("LEDGER-PROOF-BINDING: non-string {}", path.join(".")))
        })
        .collect()
}

fn add_proofs(entries: &mut BTreeMap<Key, Entry>) -> Result<(), String> {
    let proof = toml::from_str::<toml::Value>(&read_text(PROOF_RECORD)?)
        .map_err(|_| String::from("LEDGER-PROOF-BINDING: invalid proof record"))?;
    let derivation = toml::from_str::<toml::Value>(&read_text(DERIVATION_RECORD)?)
        .map_err(|_| String::from("LEDGER-PROOF-BINDING: invalid derivation record"))?;
    let approval = toml::from_str::<toml::Value>(&read_text(CORRESPONDENCE_APPROVAL)?)
        .map_err(|_| String::from("LEDGER-PROOF-BINDING: invalid approval record"))?;
    let obligations = string_array(&proof, &["property", "obligations"])?;
    let property = required_string(&proof, &["property", "id"])?;
    let implementation_path = required_string(&proof, &["implementation", "path"])?;
    let implementation_digest = required_string(&proof, &["implementation", "digest"])?;
    let proof_path = required_string(&proof, &["proof_source", "path"])?;
    let proof_digest = required_string(&proof, &["proof_source", "digest"])?;
    let negative_path = required_string(&proof, &["negative_source", "path"])?;
    let negative_diagnostic = required_string(&proof, &["negative_source", "expected_diagnostic"])?;
    let verifier = format!(
        "{} {}",
        required_string(&proof, &["verifier", "name"])?,
        required_string(&proof, &["verifier", "version"])?
    );
    let solver = required_string(&proof, &["verifier", "solver"])?;
    let configuration = required_string(&proof, &["verifier", "configuration"])?;
    let assumptions = proof
        .get("assumptions")
        .and_then(toml::Value::as_array)
        .ok_or_else(|| String::from("LEDGER-PROOF-BINDING: missing assumptions"))?
        .iter()
        .map(|item| required_string(item, &["id"]))
        .collect::<Result<Vec<_>, _>>()?
        .join(";");
    let exclusions = string_array(&proof, &["exclusions"])?.join(";");
    let scope = required_string(&proof, &["scope"])?;
    let derivation_id = required_string(&derivation, &["id"])?;
    let derivation_status = required_string(&derivation, &["status"])?;
    let assurance_effect = required_string(&derivation, &["assurance_effect"])?;
    let approval_id = required_string(&approval, &["id"])?;
    let approval_status = required_string(&approval, &["status"])?;
    let approved_derivation = required_string(&approval, &["derivation"])?;
    let correspondence =
        format!("{DERIVATION_RECORD}#{derivation_id};{CORRESPONDENCE_APPROVAL}#{approval_id}");
    let approved_obligations = string_array(&approval, &["approved_obligations"])?
        .into_iter()
        .collect::<BTreeSet<_>>();
    let blocked_obligations = string_array(&approval, &["blocked_obligations"])?
        .into_iter()
        .collect::<BTreeSet<_>>();
    let mappings = derivation
        .get("correspondence")
        .and_then(toml::Value::as_array)
        .ok_or_else(|| String::from("LEDGER-PROOF-BINDING: missing symbol correspondence"))?;
    let mut mapping_status = BTreeMap::new();
    for mapping in mappings {
        let obligation = required_string(mapping, &["obligation"])?;
        let status = required_string(mapping, &["status"])?;
        if mapping_status
            .insert(obligation.to_owned(), status.to_owned())
            .is_some()
        {
            return Err(format!(
                "LEDGER-PROOF-BINDING: duplicate mapping for {obligation}"
            ));
        }
    }
    let global_complete = required_string(&proof, &["verdict"])? == "pass"
        && required_string(&proof, &["negative_control"])? == "expected-fail"
        && derivation_status == "current"
        && approval_status == "approved"
        && approved_derivation == derivation_id
        && assurance_effect.contains("does not prove translation equivalence");
    for obligation in obligations {
        let id = format!("{property}:{}", slugify(&obligation));
        let mapping = mapping_status.get(&obligation).map(String::as_str);
        let complete = global_complete
            && mapping == Some("approved-review-only")
            && approved_obligations.contains(&obligation)
            && !blocked_obligations.contains(&obligation);
        let formal = FormalFields {
            property: property.to_owned(),
            implementation_representation: format!("{implementation_path}#{implementation_digest}"),
            correspondence: correspondence.clone(),
            proof_source: format!("{proof_path}#{proof_digest}"),
            verifier: verifier.clone(),
            solver: solver.to_owned(),
            configuration: configuration.to_owned(),
            assumptions: assumptions.clone(),
            negative_control: format!("{negative_path}#{negative_diagnostic}"),
            scope: scope.to_owned(),
            exclusions: exclusions.clone(),
        };
        let mut entry = if complete {
            satisfied(
                "proof-theorem",
                id,
                &obligation,
                "formal-proof",
                "positive proof, intended negative rejection, explicit derivation, and correspondence approval are current",
                file_digest(implementation_path)?,
                format!(
                    "{implementation_path};{proof_path};{negative_path};{PROOF_RECORD};{DERIVATION_RECORD};{CORRESPONDENCE_APPROVAL}"
                ),
            )
        } else {
            blocked(
                "proof-theorem",
                id,
                &obligation,
                "formal-proof",
                "the theorem is detached or one or more proof, negative-control, derivation, or approval bindings are incomplete",
                file_digest(implementation_path)?,
                format!("{PROOF_RECORD};{DERIVATION_RECORD};{CORRESPONDENCE_APPROVAL}"),
                PROOF_RECORD,
                CORRESPONDENCE_APPROVAL,
            )
        };
        entry.formal = Some(formal);
        entry.downstream_exclusions.clone_from(&exclusions);
        insert(entries, entry)?;
    }
    Ok(())
}

fn add_supply_chain(entries: &mut BTreeMap<Key, Entry>) -> Result<(), String> {
    for (id, path, scope) in [
        (
            "lock:cargo",
            CARGO_LOCK,
            "Cargo package versions, sources, and checksums",
        ),
        (
            "lock:devenv",
            DEVENV_LOCK,
            "Nix input revisions and content hashes",
        ),
    ] {
        insert(
            entries,
            satisfied(
                "supply-chain-input",
                id,
                scope,
                "supply-chain-policy",
                "the authoritative lock file supplies immutable pins and content digests",
                file_digest(path)?,
                path,
            ),
        )?;
    }

    let table = read_tsv(SUPPLY_EXCEPTIONS)?;
    let kind = table.column("kind")?;
    let id = table.column("id")?;
    let scope = table.column("scope")?;
    let rationale = table.column("rationale")?;
    let safety_effect = table.column("safety_effect")?;
    let replacement = table.column("replacement")?;
    for row in table.rows {
        let mut entry = satisfied(
            "supply-chain-input",
            format!("exception:{}:{}", row[kind], row[id]),
            &row[scope],
            "supply-chain-policy",
            format!("reviewed exact exception: {}", row[rationale]),
            row_digest(&row),
            SUPPLY_EXCEPTIONS,
        );
        entry.downstream_exclusions = format!(
            "{}; replacement condition: {}",
            row[safety_effect], row[replacement]
        );
        insert(entries, entry)?;
    }

    let table = read_tsv(SUPPLY_SIZE_POLICY)?;
    let component = table.column("component")?;
    let maximum = table.column("max_closure_bytes")?;
    let rationale = table.column("rationale")?;
    for row in table.rows {
        let entry = if row[maximum].parse::<u64>().is_ok_and(|value| value > 0) {
            satisfied(
                "supply-chain-input",
                format!("budget:{}", row[component]),
                format!("closure budget for {}", row[component]),
                "supply-chain-policy",
                format!(
                    "positive exact byte budget: {}; {}",
                    row[maximum], row[rationale]
                ),
                row_digest(&row),
                SUPPLY_SIZE_POLICY,
            )
        } else {
            blocked(
                "supply-chain-input",
                format!("budget:{}", row[component]),
                format!("closure budget for {}", row[component]),
                "supply-chain-policy",
                "the closure budget is missing or non-positive",
                row_digest(&row),
                SUPPLY_SIZE_POLICY,
                SUPPLY_SIZE_POLICY,
                SUPPLY_SIZE_POLICY,
            )
        };
        insert(entries, entry)?;
    }
    Ok(())
}

fn phase_result_is_current(
    text: &str,
    id: &str,
    command: &str,
    phase_handoffs_digest: &str,
    gate_source_digest: &str,
) -> bool {
    let Ok(record) = toml::from_str::<toml::Value>(text) else {
        return false;
    };
    let provider = record
        .get("provider")
        .and_then(toml::Value::as_str)
        .unwrap_or_default();
    let run_id = record
        .get("run_id")
        .and_then(toml::Value::as_str)
        .unwrap_or_default();
    let run_url = record
        .get("run_url")
        .and_then(toml::Value::as_str)
        .unwrap_or_default();
    let revision = record
        .get("revision")
        .and_then(toml::Value::as_str)
        .unwrap_or_default();
    record.get("schema").and_then(toml::Value::as_str) == Some("spec-provenance-phase-result/v2")
        && record.get("phase").and_then(toml::Value::as_str) == Some(id)
        && record.get("command").and_then(toml::Value::as_str) == Some(command)
        && record.get("status").and_then(toml::Value::as_str) == Some("green")
        && record.get("execution").and_then(toml::Value::as_str) == Some("pass")
        && record.get("provenance").and_then(toml::Value::as_str)
            == Some("independent-external-execution")
        && !provider.is_empty()
        && !run_id.is_empty()
        && run_url.starts_with("https://")
        && revision.len() == 40
        && revision
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
        && record
            .get("phase_handoffs_digest")
            .and_then(toml::Value::as_str)
            == Some(phase_handoffs_digest)
        && record
            .get("gate_source_digest")
            .and_then(toml::Value::as_str)
            == Some(gate_source_digest)
}

fn add_phases(entries: &mut BTreeMap<Key, Entry>) -> Result<(), String> {
    let value = toml::from_str::<toml::Value>(&coverage_result(9, read_text(PHASE_HANDOFFS))?)
        .map_err(|_| String::from("LEDGER-PHASE-SCHEMA: invalid handoff inventory"))?;
    if required_string(&value, &["result_schema"])? != "spec-provenance-phase-result/v2"
        || required_string(&value, &["result_provenance"])? != "independent-external-execution"
    {
        return Err(String::from(
            "LEDGER-PHASE-SCHEMA: unsupported result contract",
        ));
    }
    let required_fields = string_array(&value, &["result_required_fields"])?
        .into_iter()
        .collect::<BTreeSet<_>>();
    let expected_fields = [
        "schema",
        "phase",
        "command",
        "status",
        "execution",
        "provenance",
        "provider",
        "run_id",
        "run_url",
        "revision",
        "phase_handoffs_digest",
        "gate_source_digest",
    ]
    .into_iter()
    .map(str::to_owned)
    .collect::<BTreeSet<_>>();
    if required_fields != expected_fields {
        return Err(String::from(
            "LEDGER-PHASE-SCHEMA: incomplete result fields",
        ));
    }
    let phases = value
        .get("phase")
        .and_then(toml::Value::as_array)
        .ok_or_else(|| String::from("LEDGER-PHASE-SCHEMA: missing phases"))?;
    let self_certifying = string_array(&value, &["self_certifying_phases"])?
        .into_iter()
        .collect::<BTreeSet<_>>();
    let handoffs_digest = coverage_result(22, file_digest(PHASE_HANDOFFS))?;
    let gate_digest = file_digest(GATE_SOURCE).ok();
    for phase in phases {
        let id = required_string(phase, &["id"])?;
        let command = required_string(phase, &["command"])?;
        let success = required_string(phase, &["success"])?;
        if !self_certifying.contains(id) {
            return Err(format!(
                "LEDGER-PHASE-POLICY: local phase {id} is not self-certifying"
            ));
        }
        let result = format!("assurance/v1/phase-results/{id}.toml");
        let current = gate_digest.as_ref().is_some_and(|gate_digest| {
            fs::read_to_string(&result).ok().is_some_and(|text| {
                phase_result_is_current(&text, id, command, &handoffs_digest, gate_digest)
            })
        });
        let entry = if current {
            satisfied(
                "phase-handoff",
                id,
                success,
                "review-only",
                "the independently produced exact focused phase result binds its provider run, revision, handoff contract, gate source, command, and passing execution",
                coverage_result(10, file_digest(&result))?,
                format!("{PHASE_HANDOFFS};{result};devenv.nix"),
            )
        } else {
            not_applicable(
                "phase-handoff",
                id,
                success,
                "review-only",
                "the local final command executes the phase directly, so a separate provider result is not an acceptance prerequisite",
                coverage_result(23, file_digest(PHASE_HANDOFFS))?,
                format!("{PHASE_HANDOFFS};devenv.nix"),
                PHASE_HANDOFFS,
            )
        };
        insert(entries, entry)?;
    }
    Ok(())
}

fn validate_category_coverage(entries: &BTreeMap<Key, Entry>) -> Result<(), String> {
    let categories = entries
        .keys()
        .map(|key| key.category.as_str())
        .collect::<BTreeSet<_>>();
    if categories == CATEGORIES.into_iter().collect() {
        Ok(())
    } else {
        Err(format!("LEDGER-CATEGORY-COVERAGE: found {categories:?}"))
    }
}

fn add_category_validation(entries: &mut BTreeMap<Key, Entry>) -> Result<(), String> {
    validate_category_coverage(entries)
}

fn expected_entries() -> Result<BTreeMap<Key, Entry>, String> {
    type Collector = fn(&mut BTreeMap<Key, Entry>) -> Result<(), String>;

    let mut entries = BTreeMap::new();
    let collectors: [Collector; 13] = [
        add_requirements,
        add_schema_rows,
        add_source_targets,
        add_v2_fixture_entries,
        add_tsv_fixture_entries,
        add_properties,
        add_models,
        add_mutations,
        add_platforms,
        add_proofs,
        add_supply_chain,
        add_phases,
        add_category_validation,
    ];
    for collect in collectors {
        collect(&mut entries)?;
    }
    Ok(entries)
}

fn formal_columns(formal: Option<&FormalFields>) -> Vec<String> {
    formal.map_or_else(
        || vec!["-".to_owned(); 11],
        |value| {
            vec![
                value.property.clone(),
                value.implementation_representation.clone(),
                value.correspondence.clone(),
                value.proof_source.clone(),
                value.verifier.clone(),
                value.solver.clone(),
                value.configuration.clone(),
                value.assumptions.clone(),
                value.negative_control.clone(),
                value.scope.clone(),
                value.exclusions.clone(),
            ]
        },
    )
}

fn render(entries: &BTreeMap<Key, Entry>) -> Result<String, String> {
    let tool_digest = coverage_result(12, file_set_digest(&[TOOL_SOURCE, LEDGER_SOURCE]))?;
    let mut configuration_digests = BTreeMap::<String, String>::new();
    let mut output = String::from(HEADER);
    for entry in entries.values() {
        let configuration_digest = if let Some(digest) = &entry.configuration_digest {
            digest.clone()
        } else if let Some(digest) = configuration_digests.get(&entry.evidence_paths) {
            digest.clone()
        } else {
            let digest = coverage_result(13, evidence_configuration_digest(&entry.evidence_paths))?;
            configuration_digests.insert(entry.evidence_paths.clone(), digest.clone());
            digest
        };
        let entry_tool_digest = entry.tool_digest.as_ref().unwrap_or(&tool_digest);
        let mut fields = vec![
            LEDGER_SCHEMA_ID.to_owned(),
            entry.key.category.clone(),
            entry.key.id.clone(),
            entry.requirement_scope.clone(),
            entry.evidence_class.clone(),
            entry.disposition.clone(),
            entry.rationale.clone(),
            entry.source_digest.clone(),
            entry_tool_digest.clone(),
            configuration_digest.clone(),
            entry.evidence_paths.clone(),
            entry.expires_by_policy.clone(),
            entry.downstream_exclusions.clone(),
            entry.blocker.clone(),
            entry.approval.clone(),
            entry.result_digest.clone(),
        ];
        fields.extend(formal_columns(entry.formal.as_ref()));
        for (index, field) in fields.iter().enumerate() {
            coverage_result(14, clean_field(field, &format!("column {index}")))?;
        }
        output.push_str(&fields.join("\t"));
        output.push('\n');
    }
    Ok(output)
}

fn class_matches_category(category: &str, evidence_class: &str) -> bool {
    match category {
        "requirement" | "scenario" | "schema-row" | "fixture-operation" | "fixture-mutation" => {
            evidence_class == "point-fixture"
        }
        "source-target" => matches!(
            evidence_class,
            "point-fixture" | "structural-coverage" | "structural-source-execution"
        ),
        "property" => matches!(
            evidence_class,
            "production-property-differential" | "sampled-property"
        ),
        "finite-model" => evidence_class == "production-finite-model-conformance",
        "mutation-class" => evidence_class == "automated-mutation",
        "platform-backend" => evidence_class == "platform-conformance",
        "proof-theorem" => evidence_class == "formal-proof",
        "supply-chain-input" => evidence_class == "supply-chain-policy",
        "phase-handoff" => evidence_class == "review-only",
        _ => false,
    }
}

fn validate_semantics(text: &str, root: &Path) -> Result<Vec<Vec<String>>, String> {
    let mut lines = text.lines();
    if lines.next() != Some(HEADER.trim_end()) {
        return Err(String::from("LEDGER-HEADER: closed ledger header differs"));
    }
    let columns = HEADER.trim_end().split('\t').count();
    let mut rows = Vec::new();
    let mut keys = BTreeSet::new();
    let mut previous: Option<(String, String)> = None;
    for (index, line) in lines.enumerate() {
        let fields = line.split('\t').map(str::to_owned).collect::<Vec<_>>();
        if fields.len() != columns || fields[0] != LEDGER_SCHEMA_ID {
            return Err(format!("LEDGER-ROW-SCHEMA: row {} shape", index + 2));
        }
        if !CATEGORIES.contains(&fields[1].as_str())
            || !EVIDENCE_CLASSES.contains(&fields[4].as_str())
            || !matches!(
                fields[5].as_str(),
                "satisfied" | "not-applicable" | "unsupported" | "blocked"
            )
        {
            return Err(format!("LEDGER-VOCABULARY: row {}", index + 2));
        }
        let key = (fields[1].clone(), fields[2].clone());
        if !keys.insert(key.clone()) || previous.as_ref().is_some_and(|value| value >= &key) {
            return Err(format!("LEDGER-ORDER: row {}", index + 2));
        }
        previous = Some(key);
        for path in fields[10].split(';') {
            if path == "-" || !root.join(path).is_file() {
                return Err(format!("LEDGER-EVIDENCE-PATH: row {} `{path}`", index + 2));
            }
        }
        if fields[5] == "satisfied" {
            if [7_usize, 8, 9, 15]
                .into_iter()
                .any(|field| buoy_core::SemanticDigest::parse_canonical(&fields[field]).is_err())
            {
                return Err(format!("LEDGER-CURRENT-BINDING: row {}", index + 2));
            }
        } else if fields[5] == "not-applicable" {
            if fields[14] == "-" {
                return Err(format!("LEDGER-DISPOSITION-APPROVAL: row {}", index + 2));
            }
        } else if fields[13] == "-" || fields[14] == "-" {
            return Err(format!("LEDGER-DISPOSITION-BLOCKER: row {}", index + 2));
        }
        if !class_matches_category(&fields[1], &fields[4]) {
            return Err(format!("LEDGER-CROSS-CLASS: row {}", index + 2));
        }
        if fields[1] == "proof-theorem" {
            if fields[16..].iter().any(|field| field == "-") {
                return Err(format!("LEDGER-PROOF-BINDING: row {}", index + 2));
            }
            let correspondence = &fields[18];
            if !correspondence.contains("DERIV-")
                || !correspondence.contains("APPROVAL-")
                || correspondence.contains("digest-only")
                || correspondence == &fields[7]
            {
                return Err(format!("LEDGER-PROOF-DIGEST-ONLY: row {}", index + 2));
            }
        } else if fields[16..].iter().any(|field| field != "-") {
            return Err(format!("LEDGER-CROSS-CLASS: row {}", index + 2));
        }
        rows.push(fields);
    }
    if rows.is_empty() {
        return Err(String::from("LEDGER-EMPTY: no obligations"));
    }
    Ok(rows)
}

fn disposition_counts(rows: &[Vec<String>]) -> BTreeMap<String, usize> {
    let mut counts = BTreeMap::new();
    for row in rows {
        *counts.entry(row[5].clone()).or_insert(0) += 1;
    }
    counts
}

fn validate_materialized(
    expected: &str,
    ledger: &Path,
    root: &Path,
    write: bool,
    require_green: bool,
    announce: bool,
) -> Result<(), String> {
    if write {
        let parent = ledger
            .parent()
            .ok_or_else(|| format!("LEDGER-WRITE {}: path has no parent", ledger.display()))?;
        fs::create_dir_all(parent)
            .map_err(|error| format!("LEDGER-WRITE {}: {error}", parent.display()))?;
        fs::write(ledger, expected)
            .map_err(|error| format!("LEDGER-WRITE {}: {error}", ledger.display()))?;
    }
    let actual = fs::read_to_string(ledger)
        .map_err(|error| format!("LEDGER-READ {}: {error}", ledger.display()))?;
    let rows = coverage_result(15, validate_semantics(&actual, root))?;
    if actual != expected {
        return Err(String::from(
            "LEDGER-STALE: materialized ledger differs from exact current inventory; run --ledger-write and review every disposition",
        ));
    }
    let counts = disposition_counts(&rows);
    let blocking = counts.get("blocked").copied().unwrap_or(0)
        + counts.get("unsupported").copied().unwrap_or(0);
    if require_green && blocking > 0 {
        return Err(format!(
            "LEDGER-BLOCKING: {blocking} blocked or unsupported obligations remain"
        ));
    }
    if announce {
        println!(
            "assurance-ledger: ok entries={} satisfied={} not-applicable={} blocked={} unsupported={} require-green={require_green}",
            rows.len(),
            counts.get("satisfied").copied().unwrap_or(0),
            counts.get("not-applicable").copied().unwrap_or(0),
            counts.get("blocked").copied().unwrap_or(0),
            counts.get("unsupported").copied().unwrap_or(0),
        );
    }
    Ok(())
}

/// Validate the committed coverage result against current repository inputs.
pub fn validate_coverage_evidence() -> Result<(), String> {
    let result = read_tsv(COVERAGE_RESULT)?;
    let configuration_digest = result.column("configuration_digest")?;
    let inventory_digest = result.column("inventory_digest")?;
    let expected_configuration = file_digest(COVERAGE_CONFIGURATION)?;
    let expected_inventory = file_digest(SOURCE_INVENTORY)?;
    if result
        .rows
        .iter()
        .any(|row| row[configuration_digest] != expected_configuration)
    {
        return Err(String::from("P0-COVERAGE-STALE:configuration"));
    }
    if result
        .rows
        .iter()
        .any(|row| row[inventory_digest] != expected_inventory)
    {
        return Err(String::from("P0-COVERAGE-STALE:inventory"));
    }

    let mut entries = BTreeMap::new();
    add_source_targets(&mut entries)?;
    let blocked = entries
        .values()
        .filter(|entry| entry.disposition == "blocked")
        .map(|entry| entry.key.id.as_str())
        .collect::<Vec<_>>();
    if blocked.is_empty() {
        Ok(())
    } else {
        Err(format!("P0-COVERAGE-NOT-EXACT:{}", blocked.join(",")))
    }
}

/// Validate or materialize the exact current ledger.
pub fn validate_repository(write: bool, require_green: bool) -> Result<(), String> {
    coverage_result(16, content_digest::self_test())?;
    let entries = coverage_result(17, expected_entries())?;
    let expected = coverage_result(18, render(&entries))?;
    validate_materialized(
        &expected,
        Path::new(LEDGER),
        Path::new("."),
        write,
        require_green,
        true,
    )
}

fn replace_field(text: &str, row_index: usize, field_index: usize, value: &str) -> String {
    let mut lines = text.lines().map(str::to_owned).collect::<Vec<_>>();
    let mut fields = lines[row_index]
        .split('\t')
        .map(str::to_owned)
        .collect::<Vec<_>>();
    value.clone_into(&mut fields[field_index]);
    lines[row_index] = fields.join("\t");
    format!("{}\n", lines.join("\n"))
}

fn remove_field(text: &str, row_index: usize, field_index: usize) -> String {
    let mut lines = text.lines().map(str::to_owned).collect::<Vec<_>>();
    let mut fields = lines[row_index]
        .split('\t')
        .map(str::to_owned)
        .collect::<Vec<_>>();
    fields.remove(field_index);
    lines[row_index] = fields.join("\t");
    format!("{}\n", lines.join("\n"))
}

fn swap_rows(text: &str, first: usize, second: usize) -> String {
    let mut lines = text.lines().collect::<Vec<_>>();
    lines.swap(first, second);
    format!("{}\n", lines.join("\n"))
}

fn find_row_index(
    text: &str,
    field_index: usize,
    value: &str,
    diagnostic: &str,
) -> Result<usize, String> {
    text.lines()
        .position(|line| line.split('\t').nth(field_index) == Some(value))
        .ok_or_else(|| diagnostic.to_owned())
}

fn row_field<'a>(
    text: &'a str,
    row_index: usize,
    field_index: usize,
    diagnostic: &str,
) -> Result<&'a str, String> {
    text.lines()
        .nth(row_index)
        .and_then(|line| line.split('\t').nth(field_index))
        .ok_or_else(|| diagnostic.to_owned())
}

fn write_self_test_file(path: &Path, text: &str) {
    fs::create_dir_all(path.parent().expect("self-test path must have a parent"))
        .expect("self-test parent must be creatable");
    fs::write(path, text).expect("self-test file must be writable");
}

fn self_test_bound_execution_row(
    category: &str,
    id: &str,
    source_path: &str,
    source_text: &str,
) -> String {
    let source_digest = content_digest::bytes_binding(source_text.as_bytes());
    let tool_digest = file_set_digest(&[source_path])
        .expect("self-test bound execution row tool digest must render");
    let result = "self-test-orphan=pass";
    let result_digest = content_digest::bytes_binding(result.as_bytes());
    format!(
        "{EXECUTION_EVIDENCE_SCHEMA}\t{category}\t{id}\t{source_path}\t{source_digest}\t{source_path}\t{tool_digest}\t{source_path}\t{tool_digest}\t{result}\t{result_digest}\tpass\n"
    )
}

fn incomplete_tsv_headers(columns: &[&str]) -> Vec<String> {
    let mut headers = vec![String::new(), String::from("other\n")];
    for prefix_length in 1..columns.len() {
        headers.push(format!("{}\n", columns[..prefix_length].join("\t")));
    }
    headers
}

fn write_self_test_supply_controls() {
    write_self_test_file(Path::new(CARGO_LOCK), "cargo lock control\n");
    write_self_test_file(Path::new(DEVENV_LOCK), "devenv lock control\n");
    write_self_test_file(
        Path::new(SUPPLY_EXCEPTIONS),
        "kind\tid\tscope\trationale\tsafety_effect\treplacement\ncontrol\texception\texact scope\ttest rationale\tno effect\treplace later\n",
    );
    write_self_test_file(
        Path::new(SUPPLY_SIZE_POLICY),
        "component\tmax_closure_bytes\trationale\npositive\t1\tpositive control\nzero\t0\tzero control\ninvalid\tnot-a-number\tinvalid control\n",
    );
}

fn remove_toml_path(value: &mut toml::Value, path: &[&str]) {
    let (field, parents) = path.split_last().expect("TOML path must be nonempty");
    let mut current = value;
    for parent in parents {
        current = current
            .get_mut(*parent)
            .expect("TOML control parent must exist");
    }
    current
        .as_table_mut()
        .expect("TOML control parent must be a table")
        .remove(*field)
        .expect("TOML control field must exist");
}

fn run_self_test_proof(
    proof: &toml::Value,
    derivation: &toml::Value,
    approval: &toml::Value,
) -> Result<(), String> {
    write_self_test_file(
        Path::new(PROOF_RECORD),
        &toml::to_string(proof).expect("proof control must serialize"),
    );
    write_self_test_file(
        Path::new(DERIVATION_RECORD),
        &toml::to_string(derivation).expect("derivation control must serialize"),
    );
    write_self_test_file(
        Path::new(CORRESPONDENCE_APPROVAL),
        &toml::to_string(approval).expect("approval control must serialize"),
    );
    add_proofs(&mut BTreeMap::new())
}

fn expect_diagnostic(
    result: Result<Vec<Vec<String>>, String>,
    expected: &str,
) -> Result<(), String> {
    let error = result.map_or_else(
        |error| error,
        |_| String::from("LEDGER-NEGATIVE-CONTROL-PASSED"),
    );
    if error.starts_with(expected) {
        Ok(())
    } else {
        Err(format!(
            "LEDGER-NEGATIVE-CONTROL: expected {expected}, received {error}"
        ))
    }
}

fn read_self_test_proof_controls() -> Result<(toml::Value, toml::Value, toml::Value), String> {
    let proof = toml::from_str::<toml::Value>(&coverage_result(19, read_text(PROOF_RECORD))?)
        .expect("repository proof control must parse");
    let derivation =
        toml::from_str::<toml::Value>(&coverage_result(20, read_text(DERIVATION_RECORD))?)
            .expect("repository derivation control must parse");
    let approval =
        toml::from_str::<toml::Value>(&coverage_result(21, read_text(CORRESPONDENCE_APPROVAL))?)
            .expect("repository approval control must parse");
    Ok((proof, derivation, approval))
}

#[cfg(feature = "coverage-controls")]
fn coverage_fault_controls(entries: &BTreeMap<Key, Entry>) {
    for site in 1..=24 {
        if matches!(site, 10 | 11) {
            continue;
        }
        COVERAGE_FAULT.store(site, Ordering::Relaxed);
        let result = match site {
            1..=9 | 22..=23 => expected_entries().map(|_| ()),
            12..=14 => render(entries).map(|_| ()),
            15..=18 => validate_repository(false, false),
            19..=21 => read_self_test_proof_controls().map(|_| ()),
            _ => Err(String::from("LEDGER-COVERAGE-CONTROL:closed site range")),
        };
        assert!(result.is_err(), "coverage fault site {site} must fail");
    }
    COVERAGE_FAULT.store(0, Ordering::Relaxed);
}

/// Execute fail-closed ledger and digest-only proof negative controls.
pub fn self_test() {
    let entries = expected_entries().expect("repository entries must materialize");
    let valid = render(&entries).expect("repository ledger must render");
    let rows = validate_semantics(&valid, Path::new("."))
        .expect("rendered repository ledger must be semantically valid");
    #[cfg(feature = "coverage-controls")]
    coverage_fault_controls(&entries);

    let digest_only =
        read_text(NEGATIVE_DIGEST_ONLY).expect("digest-only negative fixture must be readable");
    expect_diagnostic(
        validate_semantics(&digest_only, Path::new(".")),
        "LEDGER-PROOF-DIGEST-ONLY",
    )
    .expect("digest-only control must produce its stable diagnostic");

    let first = valid.lines().nth(1).unwrap_or_default();
    let formal_index = find_row_index(
        &valid,
        1,
        "proof-theorem",
        "LEDGER-SELF-TEST: no formal row",
    )
    .expect("formal row must exist");
    let satisfied_index =
        find_row_index(&valid, 5, "satisfied", "LEDGER-SELF-TEST: no satisfied row")
            .expect("satisfied row must exist");
    let blocked_index = satisfied_index;
    let mut blocked_control = replace_field(&valid, blocked_index, 5, "blocked");
    blocked_control = replace_field(&blocked_control, blocked_index, 13, "explicit blocker");
    blocked_control = replace_field(
        &blocked_control,
        blocked_index,
        14,
        "explicit discharge condition",
    );
    let not_applicable_index = find_row_index(
        &valid,
        5,
        "not-applicable",
        "LEDGER-SELF-TEST: no not-applicable row",
    )
    .expect("not-applicable row must exist");
    let nonformal_index = satisfied_index;
    let formal_source_digest = row_field(
        &valid,
        formal_index,
        7,
        "LEDGER-SELF-TEST: malformed formal row",
    )
    .expect("formal source digest must exist");
    find_row_index(&valid, 1, "missing-category", "missing row control")
        .expect_err("missing row control must fail");
    row_field(&valid, usize::MAX, 7, "missing field control")
        .expect_err("missing field control must fail");
    let mut digest_equal = replace_field(&valid, formal_index, 5, "blocked");
    digest_equal = replace_field(
        &digest_equal,
        formal_index,
        7,
        "DERIV-current;APPROVAL-current",
    );
    digest_equal = replace_field(
        &digest_equal,
        formal_index,
        18,
        "DERIV-current;APPROVAL-current",
    );
    digest_equal = replace_field(&digest_equal, formal_index, 13, "explicit blocker");
    digest_equal = replace_field(&digest_equal, formal_index, 14, "explicit approval");
    let negative_controls = [
        (String::new(), Path::new("."), "LEDGER-HEADER"),
        (
            format!("{}\n", HEADER.trim_end()),
            Path::new("."),
            "LEDGER-EMPTY",
        ),
        (
            remove_field(&valid, 1, 25),
            Path::new("."),
            "LEDGER-ROW-SCHEMA",
        ),
        (
            replace_field(&valid, 1, 0, "wrong-schema"),
            Path::new("."),
            "LEDGER-ROW-SCHEMA",
        ),
        (
            replace_field(&valid, 1, 1, "unknown-category"),
            Path::new("."),
            "LEDGER-VOCABULARY",
        ),
        (
            replace_field(&valid, 1, 4, "unknown-evidence"),
            Path::new("."),
            "LEDGER-VOCABULARY",
        ),
        (
            replace_field(&valid, 1, 5, "unknown-disposition"),
            Path::new("."),
            "LEDGER-VOCABULARY",
        ),
        (format!("{valid}{first}\n"), Path::new("."), "LEDGER-ORDER"),
        (swap_rows(&valid, 1, 2), Path::new("."), "LEDGER-ORDER"),
        (
            replace_field(&valid, 1, 4, "formal-proof"),
            Path::new("."),
            "LEDGER-CROSS-CLASS",
        ),
        (
            replace_field(&valid, 1, 10, "-"),
            Path::new("."),
            "LEDGER-EVIDENCE-PATH",
        ),
        (
            valid.clone(),
            Path::new("target/hardening-ledger-missing-root"),
            "LEDGER-EVIDENCE-PATH",
        ),
        (
            replace_field(&valid, satisfied_index, 7, "stale-source"),
            Path::new("."),
            "LEDGER-CURRENT-BINDING",
        ),
        (
            replace_field(&valid, satisfied_index, 8, "stale-tool"),
            Path::new("."),
            "LEDGER-CURRENT-BINDING",
        ),
        (
            replace_field(&valid, satisfied_index, 9, "stale-configuration"),
            Path::new("."),
            "LEDGER-CURRENT-BINDING",
        ),
        (
            replace_field(&valid, satisfied_index, 15, "stale-result"),
            Path::new("."),
            "LEDGER-CURRENT-BINDING",
        ),
        (
            replace_field(&valid, not_applicable_index, 14, "-"),
            Path::new("."),
            "LEDGER-DISPOSITION-APPROVAL",
        ),
        (
            replace_field(&blocked_control, blocked_index, 13, "-"),
            Path::new("."),
            "LEDGER-DISPOSITION-BLOCKER",
        ),
        (
            replace_field(&blocked_control, blocked_index, 14, "-"),
            Path::new("."),
            "LEDGER-DISPOSITION-BLOCKER",
        ),
        (
            replace_field(&valid, formal_index, 4, "point-fixture"),
            Path::new("."),
            "LEDGER-CROSS-CLASS",
        ),
        (
            replace_field(&valid, formal_index, 18, "APPROVAL-current"),
            Path::new("."),
            "LEDGER-PROOF-DIGEST-ONLY",
        ),
        (
            replace_field(&valid, formal_index, 18, "DERIV-current"),
            Path::new("."),
            "LEDGER-PROOF-DIGEST-ONLY",
        ),
        (
            replace_field(
                &valid,
                formal_index,
                18,
                "DERIV-current;APPROVAL-current;digest-only",
            ),
            Path::new("."),
            "LEDGER-PROOF-DIGEST-ONLY",
        ),
        (
            replace_field(&valid, formal_index, 18, formal_source_digest),
            Path::new("."),
            "LEDGER-PROOF-DIGEST-ONLY",
        ),
        (digest_equal, Path::new("."), "LEDGER-PROOF-DIGEST-ONLY"),
        (
            replace_field(&valid, formal_index, 25, "-"),
            Path::new("."),
            "LEDGER-PROOF-BINDING",
        ),
        (
            replace_field(&valid, nonformal_index, 16, "cross-class-formal-field"),
            Path::new("."),
            "LEDGER-CROSS-CLASS",
        ),
    ];
    let negative_control_count = negative_controls.len() + 1;
    for (mutant, root, expected) in negative_controls {
        expect_diagnostic(validate_semantics(&mutant, root), expected)
            .expect("semantic control must produce its stable diagnostic");
    }

    let mut unsupported = replace_field(&blocked_control, blocked_index, 5, "unsupported");
    unsupported = replace_field(&unsupported, blocked_index, 13, "explicit blocker");
    unsupported = replace_field(
        &unsupported,
        blocked_index,
        14,
        "explicit approval authority",
    );
    validate_semantics(&unsupported, Path::new("."))
        .expect("unsupported disposition control must remain valid");

    let _ = disposition_counts(&rows);
    let _ = class_matches_category("unknown-category", "point-fixture");
    validate_category_coverage(&BTreeMap::new())
        .expect_err("incomplete category control must fail");

    let sandbox = PathBuf::from(format!(
        "target/hardening-ledger-self-test-{}",
        std::process::id()
    ));
    let _ = fs::remove_dir_all(&sandbox);
    fs::create_dir_all(sandbox.join("nested")).expect("ledger self-test sandbox must be creatable");
    let empty_path = sandbox.join("empty.tsv");
    let ragged_path = sandbox.join("ragged.tsv");
    let valid_path = sandbox.join("valid.tsv");
    let nested_path = sandbox.join("nested/match.txt");
    fs::write(&empty_path, "").expect("empty TSV control must be writable");
    fs::write(&ragged_path, "first\tsecond\nvalue\n").expect("ragged TSV control must be writable");
    fs::write(&valid_path, "first\tsecond\nvalue\tother\n")
        .expect("valid TSV control must be writable");
    fs::write(&nested_path, "fixture").expect("nested control must be writable");
    let empty_path = empty_path.to_string_lossy();
    let ragged_path = ragged_path.to_string_lossy();
    let valid_path = valid_path.to_string_lossy();

    read_text("target/hardening-ledger-self-test-missing")
        .expect_err("missing text control must fail");
    read_tsv(&empty_path).expect_err("empty TSV control must fail");
    read_tsv(&ragged_path).expect_err("ragged TSV control must fail");
    let table = read_tsv(&valid_path).expect("valid TSV control must parse");
    table
        .column("missing")
        .expect_err("missing column control must fail");
    let _ = table.column("first").expect("known TSV column must exist");
    file_set_digest(&["target/hardening-ledger-self-test-missing"])
        .expect_err("missing digest input control must fail");
    let _ = file_set_digest(&[valid_path.as_ref()]).expect("valid digest control must materialize");
    evidence_configuration_digest("target/hardening-ledger-self-test-missing")
        .expect_err("missing configuration input control must fail");
    for invalid in ["", "tab\tvalue", "carriage\rreturn", "line\nfeed"] {
        clean_field(invalid, "self-test").expect_err("invalid field control must fail");
    }
    let _ = clean_field("valid", "self-test").expect("valid field control must pass");

    let mut inserted = BTreeMap::new();
    let duplicate = satisfied(
        "scenario",
        "duplicate",
        "duplicate insertion control",
        "point-fixture",
        "exact duplicate insertion is rejected",
        String::from("sha-256:self-test"),
        valid_path.as_ref(),
    );
    insert(&mut inserted, duplicate.clone()).expect("first insertion control must pass");
    insert(&mut inserted, duplicate).expect_err("duplicate insertion control must fail");

    let mut collected = Vec::new();
    collect_files(&sandbox, Some("match.txt"), &mut collected)
        .expect("matching collection control must pass");
    collect_files(&sandbox, Some("absent.txt"), &mut collected)
        .expect("absent-name collection control must pass");
    collect_files(&sandbox, None, &mut collected).expect("unfiltered collection control must pass");
    collect_files(
        Path::new("target/hardening-ledger-self-test-missing"),
        None,
        &mut collected,
    )
    .expect_err("missing collection root control must fail");
    directory_entry_path(
        Err(std::io::Error::other("entry control")),
        Path::new("self-test"),
    )
    .expect_err("directory entry control must fail");
    let _ = slugify(" --Leading__and Ünicode-- ");

    let coverage_names = [
        "target_id",
        "lines_covered",
        "lines_count",
        "regions_covered",
        "regions_count",
        "functions_covered",
        "functions_count",
        "branches_covered",
        "branches_count",
        "source_digest",
        "registration",
        "freshness",
        "execution",
        "assurance_class",
    ];
    let coverage_table = Table {
        header: coverage_names.map(str::to_owned).to_vec(),
        rows: Vec::new(),
    };
    let mut coverage_row = [
        "target",
        "1",
        "1",
        "1",
        "1",
        "1",
        "1",
        "1",
        "1",
        "sha-256:self-test",
        "present",
        "current",
        "pass",
        "structural-source-execution",
    ]
    .map(str::to_owned)
    .to_vec();
    let _ = exact_coverage(&coverage_row, &coverage_table, "sha-256:self-test")
        .expect("exact coverage control must evaluate");
    for field in [1, 3, 5, 7] {
        "0".clone_into(&mut coverage_row[field]);
        let _ = exact_coverage(&coverage_row, &coverage_table, "sha-256:self-test")
            .expect("mismatched metric control must evaluate");
        "1".clone_into(&mut coverage_row[field]);
    }
    for (field, invalid) in [
        (9, "sha-256:other"),
        (10, "absent"),
        (11, "stale"),
        (12, "fail"),
        (13, "structural-coverage"),
    ] {
        let original = coverage_row[field].clone();
        invalid.clone_into(&mut coverage_row[field]);
        let _ = exact_coverage(&coverage_row, &coverage_table, "sha-256:self-test")
            .expect("mismatched binding control must evaluate");
        original.clone_into(&mut coverage_row[field]);
    }
    for prefix_length in 0..coverage_names.len() {
        exact_coverage(
            &coverage_row,
            &Table {
                header: coverage_names[..prefix_length]
                    .iter()
                    .map(|name| (*name).to_owned())
                    .collect(),
                rows: Vec::new(),
            },
            "sha-256:self-test",
        )
        .expect_err("incomplete coverage schema control must fail");
    }

    let scalar = toml::from_str::<toml::Value>("value = 'text'\narray = ['one']\n")
        .expect("scalar controls must parse");
    let mixed = toml::from_str::<toml::Value>("array = ['one', 2]\n")
        .expect("mixed-array control must parse");
    let _ = required_string(&scalar, &["value"]).expect("known string control must exist");
    required_string(&scalar, &["missing"]).expect_err("missing string control must fail");
    required_string(&scalar, &["array"]).expect_err("non-string control must fail");
    let _ = string_array(&scalar, &["array"]).expect("known array control must exist");
    string_array(&scalar, &["missing"]).expect_err("missing array control must fail");
    string_array(&scalar, &["value"]).expect_err("non-array control must fail");
    string_array(&mixed, &["array"]).expect_err("mixed-array control must fail");
    let _ = row_digest(&[]);

    let (proof_control, derivation_control, approval_control) =
        read_self_test_proof_controls().expect("repository proof controls must be readable");
    let implementation_path = required_string(&proof_control, &["implementation", "path"])
        .expect("proof implementation path must exist");
    let repository_root = std::env::current_dir().expect("repository root must be readable");
    std::env::set_current_dir(&sandbox).expect("self-test sandbox must be enterable");
    for path in [
        SCHEMA_EXECUTIONS,
        DEVENV_EXECUTIONS,
        DIGEST_EXECUTIONS,
        PRODUCT_IDENTITY_EXECUTIONS,
        QUALITY_EXECUTIONS,
        REPOSITORY_EXECUTIONS,
        ROLLBACK_EXECUTIONS,
        MIGRATION_EXECUTIONS,
        V2_EXECUTIONS,
        MODEL_EXECUTIONS,
        FALSE_CONTROL_EXECUTION,
        SCENARIO_EXECUTIONS,
    ] {
        write_self_test_file(Path::new(path), EXECUTION_HEADER);
    }
    write_self_test_file(
        Path::new(implementation_path),
        "proof implementation control\n",
    );
    write_self_test_file(
        Path::new(SCHEMA_INVENTORY),
        "category\tparent\tname\ttag\ncontrol\tparent\tname\ttag\ncontrol\tparent\tname\ttag\n",
    );
    add_schema_rows(&mut BTreeMap::new())
        .expect_err("duplicate schema-row insertion control must fail");
    write_self_test_file(
        Path::new("openspec/specs/control/spec.md"),
        "### Requirement: CONTROL-001 Synthetic requirement\n#### Scenario: Positive - Current case\n#### Scenario: Negative - Missing case\n\n### Requirement: CONTROL-002 Fully current\n#### Scenario: Positive - Current case\n",
    );
    fs::create_dir_all(HARDENING_SPECS).expect("hardening spec root must be creatable");
    write_self_test_file(
        Path::new(V2_MANIFEST),
        "fixture\tcase_id\trequirement\tspec_scenario\nfixtures/v2/control.toml\tcase-current\tCONTROL-001\tcurrent-case\nfixtures/v2/control.toml\tcase-missing\tCONTROL-001\tmissing-case\nfixtures/v2/control.toml\tcase-current\tCONTROL-002\tcurrent-case\n",
    );
    write_self_test_file(Path::new(OBSERVATIONS), "case_id\ncase-current\n");
    let mut control_entries = BTreeMap::new();
    add_requirements(&mut control_entries).expect("mixed requirement controls must be accepted");

    for malformed_spec in [
        "#### Scenario: Positive - Before requirement\n",
        "### Requirement: \n#### Scenario: Positive - Empty requirement\n",
        "### Requirement: CONTROL-003\n#### Scenario: Missing polarity\n",
        "### Requirement: CONTROL-003\n#### Scenario: Positive - One\n### Requirement: CONTROL-003\n#### Scenario: Positive - Two\n",
        "### Requirement: CONTROL-003\n",
    ] {
        write_self_test_file(Path::new("openspec/specs/control/spec.md"), malformed_spec);
        read_spec_requirements().expect_err("malformed specification control must fail");
    }
    write_self_test_file(
        Path::new("openspec/specs/control/spec.md"),
        "### Requirement: CONTROL-001 Synthetic requirement\n#### Scenario: Positive - Current case\n",
    );
    fs::remove_dir_all(HARDENING_SPECS).expect("hardening spec control root must be removable");
    read_spec_requirements().expect_err("missing hardening spec root control must fail");
    fs::create_dir_all(HARDENING_SPECS).expect("hardening spec root must be recreatable");

    write_self_test_file(
        Path::new("fixtures/v2/control.toml"),
        "[[case]]\nid = 'case-current'\noperation = 'check-current'\nmutations = ['first']\n\n[[case]]\nid = 'case-missing'\noperation = 'check-missing'\nmutations = ['second']\n",
    );
    write_self_test_file(
        Path::new(V2_MANIFEST),
        "fixture\tcase_id\trequirement\tspec_scenario\nfixtures/v2/control.toml\tcase-current\tCONTROL-001\tcurrent-case\nfixtures/v2/control.toml\tcase-missing\tCONTROL-001\tmissing-case\n",
    );
    control_entries.clear();
    add_v2_fixture_entries(&mut control_entries)
        .expect("mixed fixture-operation controls must be accepted");
    for malformed_fixture in [
        "not valid TOML = [",
        "schema = 'missing-cases'\n",
        "[[case]]\noperation = 'missing-id'\nmutations = []\n",
        "[[case]]\nid = 'missing-operation'\nmutations = []\n",
        "[[case]]\nid = 'missing-mutations'\noperation = 'check'\n",
        "[[case]]\nid = 'non-string-mutation'\noperation = 'check'\nmutations = [1]\n",
    ] {
        write_self_test_file(Path::new("fixtures/v2/control.toml"), malformed_fixture);
        add_v2_fixture_entries(&mut BTreeMap::new())
            .expect_err("malformed v2 fixture control must fail");
    }
    write_self_test_file(Path::new("fixtures/v2/control.toml"), "case = []\n");
    write_self_test_file(
        Path::new(V2_MANIFEST),
        "fixture\tcase_id\trequirement\tspec_scenario\nfixtures/v2/control.toml\tduplicate\tCONTROL-001\tcurrent-case\nfixtures/v2/control.toml\tduplicate\tCONTROL-001\tcurrent-case\n",
    );
    add_v2_fixture_entries(&mut BTreeMap::new())
        .expect_err("duplicate manifest case control must fail");

    write_self_test_file(
        Path::new("fixtures/control.tsv"),
        "case_id\toperation\tmutation\noperation-only\tread\tnone\nmutation-empty\t\t\nmutation-dash\t\t-\nmutation-real\t\trewrite\n",
    );
    write_self_test_file(Path::new("fixtures/no-case.tsv"), "value\nignored\n");
    write_self_test_file(Path::new("fixtures/not-a-table.txt"), "ignored\n");
    let execution_result = "operation=read;observed=pass";
    let execution_source = "operation-only\tread\tnone";
    let execution_source_digest = content_digest::bytes_binding(execution_source.as_bytes());
    let execution_path_digest =
        file_set_digest(&["fixtures/control.tsv"]).expect("execution control path must bind");
    let execution_result_digest = content_digest::bytes_binding(execution_result.as_bytes());
    let execution_row = format!(
        "{EXECUTION_EVIDENCE_SCHEMA}\tfixture-operation\tfixtures/control.tsv#operation-only#read\tfixtures/control.tsv\t{execution_source_digest}\tfixtures/control.tsv\t{execution_path_digest}\tfixtures/control.tsv\t{execution_path_digest}\t{execution_result}\t{execution_result_digest}\tpass\n"
    );
    let valid_execution = format!("{EXECUTION_HEADER}{execution_row}");
    let execution_configuration_binding =
        format!("\tfixtures/control.tsv\t{execution_path_digest}\t{execution_result}\t");
    write_self_test_file(Path::new(SCHEMA_EXECUTIONS), &valid_execution);
    execution_bindings(&[SCHEMA_EXECUTIONS]).expect("current execution evidence control must pass");

    assert!(path_list_digest("").is_err());
    let mut path_digest_cache_control = BTreeMap::new();
    let cached_first =
        cached_path_list_digest("fixtures/control.tsv", &mut path_digest_cache_control)
            .expect("path list digest control must render");
    let cached_second =
        cached_path_list_digest("fixtures/control.tsv", &mut path_digest_cache_control)
            .expect("cached path list digest control must render");
    assert_eq!(cached_first, cached_second);
    let source_key = Key {
        category: String::from("fixture-operation"),
        id: String::from("fixtures/control.tsv#operation-only#read"),
    };
    assert!(
        execution_digest_source(
            &source_key,
            "fixtures/control.tsv;fixtures/control.tsv",
            &mut BTreeMap::new()
        )
        .expect_err("multi-source path control must fail")
        .contains("must identify exactly one source")
    );

    write_self_test_file(
        Path::new(SCHEMA_EXECUTIONS),
        &format!(
            "{}{execution_row}",
            EXECUTION_HEADER.replacen("schema", "wrong-schema", 1)
        ),
    );
    execution_bindings(&[SCHEMA_EXECUTIONS]).expect_err("closed execution header must fail");
    let execution_row_empty_tool_path = execution_row.replacen(
        &format!("{execution_source_digest}\tfixtures/control.tsv\t"),
        &format!("{execution_source_digest}\t;fixtures/control.tsv\t"),
        1,
    );
    write_self_test_file(
        Path::new(SCHEMA_EXECUTIONS),
        &format!("{EXECUTION_HEADER}{execution_row_empty_tool_path}"),
    );
    execution_bindings(&[SCHEMA_EXECUTIONS]).expect_err("empty tool-path digest binding must fail");
    let execution_row_empty_configuration_path = execution_row.replacen(
        &format!("\t{execution_path_digest}\tfixtures/control.tsv\t"),
        &format!("\t{execution_path_digest}\t;fixtures/control.tsv\t"),
        1,
    );
    write_self_test_file(
        Path::new(SCHEMA_EXECUTIONS),
        &format!("{EXECUTION_HEADER}{execution_row_empty_configuration_path}"),
    );
    execution_bindings(&[SCHEMA_EXECUTIONS])
        .expect_err("empty configuration-path digest binding must fail");
    write_self_test_file(
        Path::new(SCHEMA_EXECUTIONS),
        &format!(
            "{EXECUTION_HEADER}{EXECUTION_EVIDENCE_SCHEMA}\tfixture-operation\tfixtures/missing-source.tsv#operation-only#read\tfixtures/missing-source.tsv\tsha-256:control\tfixtures/control.tsv\t{execution_path_digest}\tfixtures/control.tsv\t{execution_path_digest}\t{execution_result}\t{execution_result_digest}\tpass\n"
        ),
    );
    execution_bindings(&[SCHEMA_EXECUTIONS]).expect_err("missing tsv source file must fail");
    write_self_test_file(
        Path::new(SCHEMA_EXECUTIONS),
        &format!(
            "{EXECUTION_HEADER}{EXECUTION_EVIDENCE_SCHEMA}\tfixture-operation\tfixtures/no-case.tsv#operation-only#read\tfixtures/no-case.tsv\tsha-256:control\tfixtures/control.tsv\t{execution_path_digest}\tfixtures/control.tsv\t{execution_path_digest}\t{execution_result}\t{execution_result_digest}\tpass\n"
        ),
    );
    execution_bindings(&[SCHEMA_EXECUTIONS]).expect_err("missing case column must fail");
    write_self_test_file(
        Path::new(SCHEMA_EXECUTIONS),
        &valid_execution.replace("#operation-only#read", "#missing-case#read"),
    );
    execution_bindings(&[SCHEMA_EXECUTIONS]).expect_err("missing case row must fail");
    write_self_test_file(Path::new(SCHEMA_EXECUTIONS), &valid_execution);

    for invalid in [
        valid_execution.replace(&execution_source_digest, "sha-256:stale-source"),
        valid_execution.replacen(&execution_path_digest, "sha-256:stale-tool", 1),
        valid_execution.replace(
            &execution_configuration_binding,
            &format!("\tfixtures/control.tsv\tsha-256:stale-configuration\t{execution_result}\t"),
        ),
        valid_execution.replace(&execution_result_digest, "sha-256:stale-result"),
        valid_execution.replace("\tpass\n", "\tfail\n"),
        valid_execution.replacen(EXECUTION_EVIDENCE_SCHEMA, "wrong-evidence-schema", 1),
        valid_execution.replacen("fixture-operation", "wrong-category", 1),
    ] {
        write_self_test_file(Path::new(SCHEMA_EXECUTIONS), &invalid);
        execution_bindings(&[SCHEMA_EXECUTIONS])
            .expect_err("stale or nonpassing execution evidence must fail");
    }
    write_self_test_file(
        Path::new(SCHEMA_EXECUTIONS),
        &format!("{valid_execution}{execution_row}"),
    );
    execution_bindings(&[SCHEMA_EXECUTIONS]).expect_err("duplicate execution evidence must fail");
    write_self_test_file(
        Path::new(SCHEMA_EXECUTIONS),
        &valid_execution.replace("#operation-only#read", "#operation-only#unknown"),
    );
    add_tsv_fixture_entries(&mut BTreeMap::new()).expect_err("orphan execution evidence must fail");
    write_self_test_file(Path::new(SCHEMA_EXECUTIONS), &valid_execution);
    write_self_test_file(
        Path::new(NEGATIVE_DIGEST_ONLY),
        "schema\tcategory\nignored\tignored\n",
    );
    write_self_test_file(
        Path::new(V2_MANIFEST),
        "fixture\tcase_id\trequirement\tspec_scenario\n",
    );
    let operation_key = Key {
        category: String::from("fixture-operation"),
        id: String::from("fixtures/control.tsv#operation-only#read"),
    };
    let mut duplicate_operation = BTreeMap::new();
    duplicate_operation.insert(
        operation_key.clone(),
        blocked(
            "fixture-operation",
            operation_key.id,
            "duplicate operation control",
            "point-fixture",
            "duplicate operation control",
            String::from("sha-256:self-test"),
            "fixtures/control.tsv",
            "fixtures/control.tsv",
            LEDGER_SCHEMA,
        ),
    );
    add_tsv_fixture_entries(&mut duplicate_operation)
        .expect_err("duplicate operation insertion control must fail");
    let mutation_key = Key {
        category: String::from("fixture-mutation"),
        id: String::from("fixtures/control.tsv#mutation-real#3:rewrite"),
    };
    let mut duplicate_mutation = BTreeMap::new();
    duplicate_mutation.insert(
        mutation_key.clone(),
        blocked(
            "fixture-mutation",
            mutation_key.id,
            "duplicate mutation control",
            "point-fixture",
            "duplicate mutation control",
            String::from("sha-256:self-test"),
            "fixtures/control.tsv",
            "fixtures/control.tsv",
            LEDGER_SCHEMA,
        ),
    );
    add_tsv_fixture_entries(&mut duplicate_mutation)
        .expect_err("duplicate mutation insertion control must fail");
    control_entries.clear();
    add_tsv_fixture_entries(&mut control_entries).expect("TSV fixture controls must be accepted");

    write_self_test_file(Path::new(MODEL_EXECUTIONS), "");
    add_models(&mut BTreeMap::new())
        .expect_err("invalid model execution evidence must block models");
    write_self_test_file(Path::new(MODEL_EXECUTIONS), EXECUTION_HEADER);

    fs::create_dir_all("fixtures/hardening/v2/models")
        .expect("hardening model root must be creatable");
    write_self_test_file(
        Path::new("fixtures/v2/models/control.toml"),
        "state = 'control'\n",
    );
    let model_key = Key {
        category: String::from("finite-model"),
        id: String::from("v2:models:control"),
    };
    let mut duplicate_model = BTreeMap::new();
    duplicate_model.insert(
        model_key.clone(),
        blocked(
            "finite-model",
            model_key.id,
            "duplicate model control",
            "production-finite-model-conformance",
            "duplicate model control",
            String::from("sha-256:self-test"),
            "fixtures/v2/models/control.toml",
            "fixtures/v2/models/control.toml",
            LEDGER_SCHEMA,
        ),
    );
    add_models(&mut duplicate_model).expect_err("duplicate model insertion control must fail");

    write_self_test_file(
        Path::new(MODEL_EXECUTIONS),
        &format!(
            "{EXECUTION_HEADER}{}",
            self_test_bound_execution_row(
                "finite-model",
                "orphan-model",
                "fixtures/v2/models/control.toml",
                "state = 'control'\n",
            )
        ),
    );
    add_models(&mut BTreeMap::new()).expect_err("orphan model execution evidence must fail");
    write_self_test_file(Path::new(MODEL_EXECUTIONS), EXECUTION_HEADER);

    write_self_test_file(
        Path::new(PROPERTY_INVENTORY),
        "property_id\tstatus\tscope\npass\tproduction-differential\tpassing property\nfail-verdict\tsampled\tfailing verdict\nmissing-registration\tsampled\tmissing registration\naccepted-fail\tsampled\taccepted failing sample\nmissing-run\tsampled\tmissing run\n",
    );
    write_self_test_file(
        Path::new(PROPERTY_RUNS),
        "property_id\texecution\tregistration\tfreshness\tproduction_source_digest\npass\tpass\tpresent\tcurrent\tnot-applicable\nfail-verdict\tinvalid\tpresent\tcurrent\tnot-applicable\nmissing-registration\tpass\tabsent\tcurrent\tnot-applicable\naccepted-fail\tfail\tpresent\tcurrent\tsha-256:self-test\n",
    );
    write_self_test_file(Path::new(FALSE_CONTROL_EXECUTION), "");
    add_properties(&mut BTreeMap::new())
        .expect_err("invalid false-control execution evidence must block properties");
    write_self_test_file(Path::new(FALSE_CONTROL_EXECUTION), EXECUTION_HEADER);

    control_entries.clear();
    add_properties(&mut control_entries).expect("property controls must be accepted");
    let property_key = Key {
        category: String::from("property"),
        id: String::from("pass"),
    };
    let mut duplicate_property = BTreeMap::new();
    duplicate_property.insert(
        property_key.clone(),
        satisfied(
            "property",
            property_key.id,
            "duplicate property control",
            "sampled-property",
            "duplicate property control",
            String::from("sha-256:self-test"),
            PROPERTY_INVENTORY,
        ),
    );
    add_properties(&mut duplicate_property)
        .expect_err("duplicate property insertion control must fail");

    write_self_test_file(
        Path::new(FALSE_CONTROL_EXECUTION),
        &format!(
            "{EXECUTION_HEADER}{}",
            self_test_bound_execution_row(
                "property",
                "orphan-property",
                PROPERTY_INVENTORY,
                &fs::read_to_string(PROPERTY_INVENTORY)
                    .expect("property inventory control must be readable"),
            )
        ),
    );
    add_properties(&mut BTreeMap::new())
        .expect_err("orphan false-control execution evidence must fail");
    write_self_test_file(Path::new(FALSE_CONTROL_EXECUTION), EXECUTION_HEADER);

    write_self_test_file(
        Path::new(NAMED_MUTANTS),
        "id\tcategory\tsource\nmutant-control\tcontrol\tsource.rs\n",
    );
    write_self_test_file(Path::new("source.rs"), "fn control() {}\n");
    control_entries.clear();
    add_mutations(&mut control_entries).expect("stale mutation control must be accepted");
    control_entries.clear();
    add_mutations_with_current(&mut control_entries, true)
        .expect("current mutation control must be accepted");
    fs::remove_file("source.rs").expect("mutation source control must be removable");
    add_mutations_with_current(&mut BTreeMap::new(), true)
        .expect_err("missing current mutation source must fail");
    add_mutations_with_current(&mut BTreeMap::new(), false)
        .expect_err("missing stale mutation source must fail");
    write_self_test_file(Path::new("source.rs"), "fn control() {}\n");
    let mutation_key = Key {
        category: String::from("mutation-class"),
        id: String::from("mutant-control"),
    };
    let mut duplicate_mutant = BTreeMap::new();
    duplicate_mutant.insert(
        mutation_key.clone(),
        blocked(
            "mutation-class",
            mutation_key.id,
            "duplicate mutation-class control",
            "automated-mutation",
            "duplicate mutation-class control",
            String::from("sha-256:self-test"),
            "source.rs",
            NAMED_MUTANTS,
            NAMED_MUTANTS,
        ),
    );
    add_mutations_with_current(&mut duplicate_mutant, false)
        .expect_err("duplicate mutation-class insertion control must fail");

    write_self_test_supply_controls();
    control_entries.clear();
    add_supply_chain(&mut control_entries).expect("supply-chain controls must be accepted");

    fs::remove_file(CARGO_LOCK).expect("cargo lock control must be removable");
    add_supply_chain(&mut BTreeMap::new())
        .expect_err("missing supply-chain lock digest control must fail");
    write_self_test_supply_controls();

    for (path, columns) in [
        (
            SUPPLY_EXCEPTIONS,
            &[
                "kind",
                "id",
                "scope",
                "rationale",
                "safety_effect",
                "replacement",
            ][..],
        ),
        (
            SUPPLY_SIZE_POLICY,
            &["component", "max_closure_bytes", "rationale"][..],
        ),
    ] {
        for header in incomplete_tsv_headers(columns) {
            write_self_test_supply_controls();
            write_self_test_file(Path::new(path), &header);
            add_supply_chain(&mut BTreeMap::new())
                .expect_err("incomplete supply-chain table must fail");
        }
    }
    for (id, scope) in [
        (
            "lock:cargo",
            "Cargo package versions, sources, and checksums",
        ),
        ("lock:devenv", "Nix input revisions and content hashes"),
        ("exception:control:exception", "exact scope"),
        ("budget:positive", "closure budget for positive"),
    ] {
        write_self_test_supply_controls();
        let supply_key = Key {
            category: String::from("supply-chain-input"),
            id: id.to_owned(),
        };
        let mut duplicate_supply = BTreeMap::new();
        duplicate_supply.insert(
            supply_key.clone(),
            satisfied(
                "supply-chain-input",
                supply_key.id,
                scope,
                "supply-chain-policy",
                "duplicate supply-chain control",
                String::from("sha-256:self-test"),
                CARGO_LOCK,
            ),
        );
        add_supply_chain(&mut duplicate_supply)
            .expect_err("duplicate supply-chain insertion control must fail");
    }
    write_self_test_supply_controls();

    run_self_test_proof(&proof_control, &derivation_control, &approval_control)
        .expect("current proof control must be accepted");
    for missing_record in [PROOF_RECORD, DERIVATION_RECORD, CORRESPONDENCE_APPROVAL] {
        run_self_test_proof(&proof_control, &derivation_control, &approval_control)
            .expect("proof controls must be restorable");
        fs::remove_file(missing_record).expect("proof record control must be removable");
        add_proofs(&mut BTreeMap::new()).expect_err("missing proof record control must fail");
    }
    for path in [
        &["property", "obligations"][..],
        &["property", "id"][..],
        &["implementation", "path"][..],
        &["implementation", "digest"][..],
        &["proof_source", "path"][..],
        &["proof_source", "digest"][..],
        &["negative_source", "path"][..],
        &["negative_source", "expected_diagnostic"][..],
        &["verifier", "name"][..],
        &["verifier", "version"][..],
        &["verifier", "solver"][..],
        &["verifier", "configuration"][..],
        &["exclusions"][..],
        &["scope"][..],
        &["verdict"][..],
        &["negative_control"][..],
    ] {
        let mut mutant = proof_control.clone();
        remove_toml_path(&mut mutant, path);
        run_self_test_proof(&mutant, &derivation_control, &approval_control)
            .expect_err("missing proof field control must fail");
    }
    for path in [&["id"][..], &["status"][..], &["assurance_effect"][..]] {
        let mut mutant = derivation_control.clone();
        remove_toml_path(&mut mutant, path);
        run_self_test_proof(&proof_control, &mutant, &approval_control)
            .expect_err("missing derivation field control must fail");
    }
    for path in [
        &["id"][..],
        &["status"][..],
        &["derivation"][..],
        &["approved_obligations"][..],
        &["blocked_obligations"][..],
    ] {
        let mut mutant = approval_control.clone();
        remove_toml_path(&mut mutant, path);
        run_self_test_proof(&proof_control, &derivation_control, &mutant)
            .expect_err("missing approval field control must fail");
    }
    let mut mutant = proof_control.clone();
    remove_toml_path(
        mutant
            .get_mut("assumptions")
            .and_then(toml::Value::as_array_mut)
            .and_then(|items| items.first_mut())
            .expect("proof assumption control must exist"),
        &["id"],
    );
    run_self_test_proof(&mutant, &derivation_control, &approval_control)
        .expect_err("missing assumption ID control must fail");
    for field in ["obligation", "status"] {
        let mut mutant = derivation_control.clone();
        remove_toml_path(
            mutant
                .get_mut("correspondence")
                .and_then(toml::Value::as_array_mut)
                .and_then(|items| items.first_mut())
                .expect("proof mapping control must exist"),
            &[field],
        );
        run_self_test_proof(&proof_control, &mutant, &approval_control)
            .expect_err("missing mapping field control must fail");
    }
    fs::remove_file(implementation_path).expect("proof implementation control must be removable");
    run_self_test_proof(&proof_control, &derivation_control, &approval_control)
        .expect_err("missing satisfied proof implementation must fail");
    let mut incomplete_proof = proof_control.clone();
    incomplete_proof["verdict"] = toml::Value::String(String::from("fail"));
    run_self_test_proof(&incomplete_proof, &derivation_control, &approval_control)
        .expect_err("missing blocked proof implementation must fail");
    write_self_test_file(
        Path::new(implementation_path),
        "proof implementation control\n",
    );
    run_self_test_proof(&proof_control, &derivation_control, &approval_control)
        .expect("proof controls must be restorable");
    let mut duplicate_proof = BTreeMap::new();
    add_proofs(&mut duplicate_proof).expect("first proof insertion control must pass");
    add_proofs(&mut duplicate_proof).expect_err("duplicate proof insertion control must fail");

    for (record, malformed) in [
        (PROOF_RECORD, "not valid proof TOML = ["),
        (DERIVATION_RECORD, "not valid derivation TOML = ["),
        (CORRESPONDENCE_APPROVAL, "not valid approval TOML = ["),
    ] {
        run_self_test_proof(&proof_control, &derivation_control, &approval_control)
            .expect("proof controls must be restorable");
        write_self_test_file(Path::new(record), malformed);
        add_proofs(&mut BTreeMap::new()).expect_err("malformed proof control must fail");
    }
    let mut mutant = proof_control.clone();
    mutant
        .as_table_mut()
        .expect("proof control must be a table")
        .remove("assumptions");
    run_self_test_proof(&mutant, &derivation_control, &approval_control)
        .expect_err("missing proof assumptions control must fail");
    let mut mutant = derivation_control.clone();
    mutant
        .as_table_mut()
        .expect("derivation control must be a table")
        .remove("correspondence");
    run_self_test_proof(&proof_control, &mutant, &approval_control)
        .expect_err("missing proof correspondence control must fail");
    let mut mutant = derivation_control.clone();
    let mappings = mutant
        .get_mut("correspondence")
        .and_then(toml::Value::as_array_mut)
        .expect("derivation mappings must be an array");
    mappings.push(mappings[0].clone());
    run_self_test_proof(&proof_control, &mutant, &approval_control)
        .expect_err("duplicate proof mapping control must fail");

    for (record, path, value) in [
        ("proof", &["verdict"][..], "fail"),
        ("proof", &["negative_control"][..], "unexpected-pass"),
        ("derivation", &["status"][..], "stale"),
        (
            "derivation",
            &["assurance_effect"][..],
            "claims equivalence",
        ),
        ("approval", &["status"][..], "pending"),
        ("approval", &["derivation"][..], "other-derivation"),
    ] {
        let mut proof = proof_control.clone();
        let mut derivation = derivation_control.clone();
        let mut approval = approval_control.clone();
        let target = match record {
            "proof" => &mut proof,
            "derivation" => &mut derivation,
            _ => &mut approval,
        };
        let mut current = target;
        for component in path {
            current = current
                .get_mut(*component)
                .expect("proof control path must exist");
        }
        *current = toml::Value::String(value.to_owned());
        run_self_test_proof(&proof, &derivation, &approval)
            .expect("incomplete proof control must materialize blocked entries");
    }
    let mut missing_mapping = derivation_control.clone();
    missing_mapping
        .get_mut("correspondence")
        .and_then(toml::Value::as_array_mut)
        .expect("derivation mappings must be an array")
        .remove(0);
    run_self_test_proof(&proof_control, &missing_mapping, &approval_control)
        .expect("missing mapping control must materialize blocked entry");
    let mut unapproved = approval_control.clone();
    unapproved
        .get_mut("approved_obligations")
        .and_then(toml::Value::as_array_mut)
        .expect("approved obligations must be an array")
        .remove(0);
    run_self_test_proof(&proof_control, &derivation_control, &unapproved)
        .expect("unapproved obligation control must materialize blocked entry");
    let mut explicitly_blocked = approval_control.clone();
    let obligation = explicitly_blocked
        .get("approved_obligations")
        .and_then(toml::Value::as_array)
        .and_then(|values| values.first())
        .cloned()
        .expect("approved obligation control must exist");
    explicitly_blocked
        .get_mut("blocked_obligations")
        .and_then(toml::Value::as_array_mut)
        .expect("blocked obligations must be an array")
        .push(obligation);
    run_self_test_proof(&proof_control, &derivation_control, &explicitly_blocked)
        .expect("blocked obligation control must materialize blocked entry");

    write_self_test_file(
        Path::new(PHASE_HANDOFFS),
        "result_schema = 'spec-provenance-phase-result/v2'\nresult_provenance = 'independent-external-execution'\nresult_required_fields = ['schema', 'phase', 'command', 'status', 'execution', 'provenance', 'provider', 'run_id', 'run_url', 'revision', 'phase_handoffs_digest', 'gate_source_digest']\nself_certifying_phases = ['current', 'missing', 'self-certifying']\n\n[[phase]]\nid = 'current'\ncommand = 'control current'\nsuccess = 'current phase succeeds'\n\n[[phase]]\nid = 'missing'\ncommand = 'control missing'\nsuccess = 'missing local phase record is optional'\n\n[[phase]]\nid = 'self-certifying'\ncommand = 'control self certifying'\nsuccess = 'self certifying phase does not block itself'\n",
    );
    write_self_test_file(Path::new(GATE_SOURCE), "gate control\n");
    let phase_handoffs_digest = file_digest(PHASE_HANDOFFS).expect("phase handoff digest control");
    let gate_source_digest = file_digest(GATE_SOURCE).expect("gate source digest control");
    let valid_phase_result = format!(
        "schema = 'spec-provenance-phase-result/v2'\nphase = 'current'\ncommand = 'control current'\nstatus = 'green'\nexecution = 'pass'\nprovenance = 'independent-external-execution'\nprovider = 'control-ci'\nrun_id = '123'\nrun_url = 'https://ci.invalid/runs/123'\nrevision = '0123456789abcdef0123456789abcdef01234567'\nphase_handoffs_digest = '{phase_handoffs_digest}'\ngate_source_digest = '{gate_source_digest}'\n"
    );
    write_self_test_file(
        Path::new("assurance/v1/phase-results/current.toml"),
        &valid_phase_result,
    );
    #[cfg(feature = "coverage-controls")]
    {
        COVERAGE_FAULT.store(10, Ordering::Relaxed);
        add_phases(&mut BTreeMap::new()).expect_err("phase digest coverage control must fail");
        COVERAGE_FAULT.store(0, Ordering::Relaxed);
    }
    control_entries.clear();
    add_phases(&mut control_entries).expect("phase controls must be accepted");
    for (id, expected) in [
        ("current", "satisfied"),
        ("missing", "not-applicable"),
        ("self-certifying", "not-applicable"),
    ] {
        assert_eq!(
            control_entries
                .get(&Key {
                    category: String::from("phase-handoff"),
                    id: id.to_owned(),
                })
                .expect("phase control entry must exist")
                .disposition,
            expected,
            "phase {id} disposition control"
        );
    }
    for id in ["current", "missing"] {
        let phase_key = Key {
            category: String::from("phase-handoff"),
            id: id.to_owned(),
        };
        let mut duplicate_phase = BTreeMap::new();
        duplicate_phase.insert(
            phase_key.clone(),
            blocked(
                "phase-handoff",
                phase_key.id,
                "duplicate phase control",
                "review-only",
                "duplicate phase control",
                String::from("sha-256:self-test"),
                PHASE_HANDOFFS,
                PHASE_HANDOFFS,
                PHASE_HANDOFFS,
            ),
        );
        add_phases(&mut duplicate_phase).expect_err("duplicate phase insertion control must fail");
    }
    let invalid_phase_results = [
        String::from("not valid phase result TOML = ["),
        valid_phase_result.replacen("spec-provenance-phase-result/v2", "wrong", 1),
        valid_phase_result.replacen("phase = 'current'", "phase = 'wrong'", 1),
        valid_phase_result.replacen("command = 'control current'", "command = 'wrong'", 1),
        valid_phase_result.replacen("status = 'green'", "status = 'red'", 1),
        valid_phase_result.replacen("execution = 'pass'", "execution = 'fail'", 1),
        valid_phase_result.replacen(
            "provenance = 'independent-external-execution'",
            "provenance = 'registration-only'",
            1,
        ),
        valid_phase_result.replacen("provider = 'control-ci'", "provider = ''", 1),
        valid_phase_result.replacen("run_id = '123'", "run_id = ''", 1),
        valid_phase_result.replacen("https://ci.invalid", "file://local", 1),
        valid_phase_result.replacen(
            "0123456789abcdef0123456789abcdef01234567",
            "not-a-revision",
            1,
        ),
        valid_phase_result.replacen(
            "0123456789abcdef0123456789abcdef01234567",
            "g123456789abcdef0123456789abcdef01234567",
            1,
        ),
        valid_phase_result.replace("provider = 'control-ci'\n", ""),
        valid_phase_result.replace("run_id = '123'\n", ""),
        valid_phase_result.replace("run_url = 'https://ci.invalid/runs/123'\n", ""),
        valid_phase_result.replace(
            "revision = '0123456789abcdef0123456789abcdef01234567'\n",
            "",
        ),
        valid_phase_result.replacen(&phase_handoffs_digest, "sha-256:stale", 1),
        valid_phase_result.replacen(&gate_source_digest, "sha-256:stale", 1),
    ];
    for invalid_result in &invalid_phase_results {
        write_self_test_file(
            Path::new("assurance/v1/phase-results/current.toml"),
            invalid_result,
        );
        add_phases(&mut BTreeMap::new())
            .expect("invalid optional phase result must remain nonblocking");
    }
    write_self_test_file(
        Path::new("assurance/v1/phase-results/current.toml"),
        &valid_phase_result,
    );
    fs::remove_file(GATE_SOURCE).expect("gate source control must be removable");
    add_phases(&mut BTreeMap::new())
        .expect("missing gate digest must keep optional phase records nonblocking");
    write_self_test_file(Path::new(GATE_SOURCE), "gate control\n");
    let phase_result_contract = "result_schema = 'spec-provenance-phase-result/v2'\nresult_provenance = 'independent-external-execution'\nresult_required_fields = ['schema', 'phase', 'command', 'status', 'execution', 'provenance', 'provider', 'run_id', 'run_url', 'revision', 'phase_handoffs_digest', 'gate_source_digest']\nself_certifying_phases = []\n";
    for phase in [
        format!("{phase_result_contract}[[phase]]\ncommand = 'missing id'\nsuccess = 'fails'\n"),
        format!("{phase_result_contract}[[phase]]\nid = 'missing-command'\nsuccess = 'fails'\n"),
        format!("{phase_result_contract}[[phase]]\nid = 'missing-success'\ncommand = 'control'\n"),
        format!(
            "{phase_result_contract}[[phase]]\nid = 'not-self-certifying'\ncommand = 'control'\nsuccess = 'fails'\n"
        ),
        String::from(
            "result_schema = 'spec-provenance-phase-result/v2'\nresult_provenance = 'independent-external-execution'\nresult_required_fields = ['schema', 'phase', 'command', 'status', 'execution', 'provenance', 'provider', 'run_id', 'run_url', 'revision', 'phase_handoffs_digest', 'gate_source_digest']\n[[phase]]\nid = 'control'\ncommand = 'control'\nsuccess = 'missing self-certifying declaration fails'\n",
        ),
        String::from(
            "result_schema = 'wrong'\nresult_provenance = 'independent-external-execution'\nresult_required_fields = []\n[[phase]]\nid = 'control'\ncommand = 'control'\nsuccess = 'fails'\n",
        ),
        String::from(
            "result_schema = 'spec-provenance-phase-result/v2'\nresult_provenance = 'wrong'\nresult_required_fields = []\n[[phase]]\nid = 'control'\ncommand = 'control'\nsuccess = 'fails'\n",
        ),
        String::from(
            "result_schema = 'spec-provenance-phase-result/v2'\nresult_provenance = 'independent-external-execution'\nresult_required_fields = []\n[[phase]]\nid = 'control'\ncommand = 'control'\nsuccess = 'fails'\n",
        ),
        String::from(
            "result_schema = 'spec-provenance-phase-result/v2'\nresult_required_fields = []\n[[phase]]\nid = 'control'\ncommand = 'control'\nsuccess = 'fails'\n",
        ),
        String::from(
            "result_schema = 'spec-provenance-phase-result/v2'\nresult_provenance = 'independent-external-execution'\n[[phase]]\nid = 'control'\ncommand = 'control'\nsuccess = 'fails'\n",
        ),
        String::from(
            "result_schema = 'spec-provenance-phase-result/v2'\nresult_provenance = 'independent-external-execution'\nresult_required_fields = ['schema', 'phase', 'command', 'status', 'execution', 'provenance', 'provider', 'run_id', 'run_url', 'revision', 'phase_handoffs_digest', 'gate_source_digest']\n",
        ),
    ] {
        write_self_test_file(Path::new(PHASE_HANDOFFS), &phase);
        add_phases(&mut BTreeMap::new()).expect_err("incomplete phase control must fail");
    }

    write_self_test_file(Path::new(OBSERVATIONS), "");
    observed_fixture_cases().expect_err("unreadable observations control must fail");
    write_self_test_file(Path::new(OBSERVATIONS), "other\n");
    observed_fixture_cases().expect_err("missing observation column control must fail");
    write_self_test_file(Path::new(OBSERVATIONS), "case_id\ncase-current\n");
    observed_fixture_cases().expect("observed fixture cases control must render");

    for header in [
        "",
        "other\n",
        "requirement\n",
        "requirement\tspec_scenario\n",
    ] {
        write_self_test_file(Path::new(V2_MANIFEST), header);
        v2_scenario_cases().expect_err("incomplete scenario manifest control must fail");
    }
    write_self_test_file(
        Path::new(V2_MANIFEST),
        "fixture\tcase_id\trequirement\tspec_scenario\nfixtures/v2/control.toml\tcase-current\tCONTROL-001\tcurrent-case\nfixtures/v2/control.toml\tcase-nested\tCONTROL-001\tgroup/nested-case\n",
    );
    v2_scenario_cases().expect("v2 scenario cases control must render");

    for header in [
        "",
        "other\n",
        "category\n",
        "category\tparent\n",
        "category\tparent\tname\n",
    ] {
        write_self_test_file(Path::new(SCHEMA_INVENTORY), header);
        add_schema_rows(&mut BTreeMap::new()).expect_err("incomplete schema inventory must fail");
    }

    for header in ["", "other\n"] {
        write_self_test_file(Path::new(COVERAGE_RESULT), header);
        coverage_rows().expect_err("incomplete coverage result must fail");
    }

    let valid_inventory = "property_id\tstatus\tscope\npass\tsampled\tcontrol\n";
    let valid_runs = "property_id\texecution\tregistration\tfreshness\tproduction_source_digest\npass\tpass\tpresent\tcurrent\tnot-applicable\n";
    for header in ["", "other\n", "property_id\n", "property_id\tstatus\n"] {
        write_self_test_file(Path::new(PROPERTY_INVENTORY), header);
        write_self_test_file(Path::new(PROPERTY_RUNS), valid_runs);
        add_properties(&mut BTreeMap::new()).expect_err("incomplete property inventory must fail");
    }
    for header in [
        "",
        "other\n",
        "property_id\n",
        "property_id\texecution\n",
        "property_id\texecution\tregistration\n",
        "property_id\texecution\tregistration\tfreshness\n",
    ] {
        write_self_test_file(Path::new(PROPERTY_INVENTORY), valid_inventory);
        write_self_test_file(Path::new(PROPERTY_RUNS), header);
        add_properties(&mut BTreeMap::new()).expect_err("incomplete property run must fail");
    }

    for header in ["", "other\n", "id\n", "id\tcategory\n"] {
        write_self_test_file(Path::new(NAMED_MUTANTS), header);
        add_mutations_with_current(&mut BTreeMap::new(), false)
            .expect_err("incomplete mutation inventory must fail");
    }

    write_self_test_file(
        Path::new(SOURCE_INVENTORY),
        "target_id\tpath\tcoverage_class\tdisposition\trationale\ncontrol\tsource.rs\tcontrol\tincluded-fixture\tcontrol\n",
    );
    write_self_test_file(
        Path::new(COVERAGE_RESULT),
        "target_id\tlines_covered\tlines_count\tregions_covered\tregions_count\tfunctions_covered\tfunctions_count\tbranches_covered\tbranches_count\tsource_digest\tregistration\tfreshness\texecution\tassurance_class\n",
    );
    write_self_test_file(Path::new(SOURCE_INVENTORY), "");
    add_source_targets(&mut BTreeMap::new()).expect_err("missing source inventory must fail");
    write_self_test_file(
        Path::new(SOURCE_INVENTORY),
        "target_id\tpath\tcoverage_class\tdisposition\trationale\ncontrol\tsource.rs\tcontrol\tincluded\tcontrol\n",
    );
    write_self_test_file(Path::new(COVERAGE_RESULT), "other\n");
    add_source_targets(&mut BTreeMap::new()).expect_err("unindexed coverage result must fail");
    write_self_test_file(
        Path::new(COVERAGE_RESULT),
        "target_id\tlines_covered\tlines_count\tregions_covered\tregions_count\tfunctions_covered\tfunctions_count\tbranches_covered\tbranches_count\tsource_digest\tregistration\tfreshness\texecution\tassurance_class\n",
    );
    write_self_test_file(
        Path::new(SOURCE_INVENTORY),
        "target_id\tpath\tcoverage_class\tdisposition\trationale\nmissing\tmissing.rs\tcontrol\tincluded-fixture\tcontrol\n",
    );
    add_source_targets(&mut BTreeMap::new()).expect_err("missing source target must fail");
    write_self_test_file(
        Path::new(SOURCE_INVENTORY),
        "target_id\tpath\tcoverage_class\tdisposition\trationale\ncontrol\tsource.rs\tcontrol\tincluded-fixture\tcontrol\n",
    );
    let source_key = Key {
        category: String::from("source-target"),
        id: String::from("control"),
    };
    let mut duplicate_source = BTreeMap::new();
    duplicate_source.insert(
        source_key.clone(),
        satisfied(
            "source-target",
            source_key.id,
            "duplicate source control",
            "point-fixture",
            "duplicate source control",
            String::from("sha-256:self-test"),
            "source.rs",
        ),
    );
    add_source_targets(&mut duplicate_source)
        .expect_err("duplicate source insertion control must fail");
    let source_columns = [
        "target_id",
        "path",
        "coverage_class",
        "disposition",
        "rationale",
    ];
    for prefix_length in 0..source_columns.len() {
        write_self_test_file(
            Path::new(SOURCE_INVENTORY),
            &format!("{}\n", source_columns[..prefix_length].join("\t")),
        );
        add_source_targets(&mut BTreeMap::new())
            .expect_err("incomplete source inventory must fail");
    }
    write_self_test_file(
        Path::new(SOURCE_INVENTORY),
        "target_id\tpath\tcoverage_class\tdisposition\trationale\ncontrol\tsource.rs\tcontrol\tincluded\tcontrol\n",
    );
    write_self_test_file(Path::new(COVERAGE_RESULT), "");
    add_source_targets(&mut BTreeMap::new()).expect_err("missing coverage result must fail");

    write_self_test_file(
        Path::new(SOURCE_INVENTORY),
        "target_id\tpath\tcoverage_class\tdisposition\trationale\ncontrol\tsource.rs\tcontrol\tincluded\tcontrol\n",
    );
    write_self_test_file(
        Path::new(COVERAGE_RESULT),
        "target_id\tlines_covered\tlines_count\tregions_covered\tregions_count\tfunctions_covered\tfunctions_count\tbranches_covered\tbranches_count\tsource_digest\tregistration\tfreshness\texecution\tassurance_class\ncontrol\t0\t1\t1\t1\t1\t1\t1\t1\tsha-256:mismatch\tpresent\tcurrent\tpass\tstructural-source-execution\n",
    );
    let mut blocked_source = BTreeMap::new();
    add_source_targets(&mut blocked_source).expect("blocked source-target control must render");
    let blocked_source_entry = blocked_source
        .get(&Key {
            category: String::from("source-target"),
            id: String::from("control"),
        })
        .expect("blocked source-target entry must exist");
    assert_eq!(blocked_source_entry.disposition, "blocked");
    assert_eq!(
        blocked_source_entry.rationale,
        "the source target is absent, stale, or not exact in the current coverage result"
    );
    write_self_test_file(
        Path::new(SOURCE_INVENTORY),
        "target_id\tpath\tcoverage_class\tdisposition\trationale\ncontrol\tsource.rs\tcontrol\tincluded-manual\tcontrol\n",
    );
    let mut manual_source = BTreeMap::new();
    add_source_targets(&mut manual_source).expect("manual source-target control must render");
    assert_eq!(
        manual_source
            .get(&Key {
                category: String::from("source-target"),
                id: String::from("control"),
            })
            .expect("manual source-target entry must exist")
            .disposition,
        "not-applicable"
    );
    write_self_test_file(
        Path::new(SOURCE_INVENTORY),
        "target_id\tpath\tcoverage_class\tdisposition\trationale\ncontrol\tsource.rs\tcontrol\tincluded\tcontrol\n",
    );
    let source_digest = file_digest("source.rs").expect("source control digest must render");
    write_self_test_file(
        Path::new(COVERAGE_RESULT),
        &format!(
            "target_id\tlines_covered\tlines_count\tregions_covered\tregions_count\tfunctions_covered\tfunctions_count\tbranches_covered\tbranches_count\tsource_digest\tregistration\tfreshness\texecution\tassurance_class\ncontrol\t1\t1\t1\t1\t1\t1\t1\t1\t{source_digest}\tpresent\tcurrent\tpass\tstructural-source-execution\n"
        ),
    );
    let mut satisfied_source = BTreeMap::new();
    add_source_targets(&mut satisfied_source).expect("exact source-target control must render");
    assert_eq!(
        satisfied_source
            .get(&Key {
                category: String::from("source-target"),
                id: String::from("control"),
            })
            .expect("exact source-target entry must exist")
            .disposition,
        "satisfied"
    );

    write_self_test_file(
        Path::new(COVERAGE_CONFIGURATION),
        "schema = 'coverage-control'\n",
    );
    let configuration_digest =
        file_digest(COVERAGE_CONFIGURATION).expect("coverage configuration must bind");
    let inventory_digest = file_digest(SOURCE_INVENTORY).expect("source inventory must bind");
    let recorded_coverage = format!(
        "target_id\tlines_covered\tlines_count\tregions_covered\tregions_count\tfunctions_covered\tfunctions_count\tbranches_covered\tbranches_count\tsource_digest\tregistration\tfreshness\texecution\tassurance_class\tconfiguration_digest\tinventory_digest\ncontrol\t1\t1\t1\t1\t1\t1\t1\t1\t{source_digest}\tpresent\tcurrent\tpass\tstructural-source-execution\t{configuration_digest}\t{inventory_digest}\n"
    );
    write_self_test_file(Path::new(COVERAGE_RESULT), &recorded_coverage);
    validate_coverage_evidence().expect("current exact coverage evidence must pass");

    fs::remove_file(COVERAGE_RESULT).expect("remove coverage result control");
    validate_coverage_evidence().expect_err("missing coverage result must fail");
    write_self_test_file(Path::new(COVERAGE_RESULT), &recorded_coverage);
    write_self_test_file(Path::new(COVERAGE_RESULT), "other\n");
    validate_coverage_evidence().expect_err("missing configuration column must fail");
    write_self_test_file(Path::new(COVERAGE_RESULT), "configuration_digest\n");
    validate_coverage_evidence().expect_err("missing inventory column must fail");
    write_self_test_file(Path::new(COVERAGE_RESULT), &recorded_coverage);

    fs::remove_file(COVERAGE_CONFIGURATION).expect("remove coverage configuration control");
    validate_coverage_evidence().expect_err("missing coverage configuration must fail");
    write_self_test_file(
        Path::new(COVERAGE_CONFIGURATION),
        "schema = 'coverage-control'\n",
    );
    fs::remove_file(SOURCE_INVENTORY).expect("remove source inventory control");
    validate_coverage_evidence().expect_err("missing source inventory must fail");
    write_self_test_file(
        Path::new(SOURCE_INVENTORY),
        "target_id\tpath\tcoverage_class\tdisposition\trationale\ncontrol\tsource.rs\tcontrol\tincluded\tcontrol\n",
    );

    write_self_test_file(Path::new(SOURCE_INVENTORY), "other\n");
    let invalid_inventory_digest =
        file_digest(SOURCE_INVENTORY).expect("invalid source inventory must bind");
    write_self_test_file(
        Path::new(COVERAGE_RESULT),
        &recorded_coverage.replace(&inventory_digest, &invalid_inventory_digest),
    );
    validate_coverage_evidence().expect_err("invalid source inventory must fail");
    write_self_test_file(
        Path::new(SOURCE_INVENTORY),
        "target_id\tpath\tcoverage_class\tdisposition\trationale\ncontrol\tsource.rs\tcontrol\tincluded\tcontrol\n",
    );

    write_self_test_file(
        Path::new(COVERAGE_RESULT),
        &recorded_coverage.replace(&configuration_digest, "sha-256:stale-configuration"),
    );
    assert_eq!(
        validate_coverage_evidence().expect_err("stale coverage configuration must fail"),
        "P0-COVERAGE-STALE:configuration"
    );
    write_self_test_file(
        Path::new(COVERAGE_RESULT),
        &recorded_coverage.replace(&inventory_digest, "sha-256:stale-inventory"),
    );
    assert_eq!(
        validate_coverage_evidence().expect_err("stale source inventory must fail"),
        "P0-COVERAGE-STALE:inventory"
    );
    write_self_test_file(
        Path::new(COVERAGE_RESULT),
        &recorded_coverage.replace(&source_digest, "sha-256:stale-source"),
    );
    assert_eq!(
        validate_coverage_evidence().expect_err("stale source result must fail"),
        "P0-COVERAGE-NOT-EXACT:control"
    );

    for header in ["", "other\n", "fixture\n"] {
        write_self_test_file(Path::new(V2_MANIFEST), header);
        add_v2_fixture_entries(&mut BTreeMap::new()).expect_err("incomplete v2 manifest must fail");
    }
    write_self_test_file(
        Path::new(V2_MANIFEST),
        "fixture\tcase_id\trequirement\tspec_scenario\nfixtures/v2/control.toml\tcase-current\tCONTROL-001\tcurrent-case\n",
    );
    write_self_test_file(
        Path::new("fixtures/v2/control.toml"),
        "[[case]]\nid = 'case-current'\noperation = 'check-current'\nmutations = ['first']\n",
    );
    write_self_test_file(Path::new(V2_EXECUTIONS), "");
    add_v2_fixture_entries(&mut BTreeMap::new())
        .expect_err("invalid execution evidence must block v2 fixtures");
    write_self_test_file(Path::new(V2_EXECUTIONS), EXECUTION_HEADER);
    write_self_test_file(
        Path::new(V2_MANIFEST),
        "fixture\tcase_id\trequirement\tspec_scenario\nfixtures/v2/missing.toml\tmissing\tCONTROL-001\tcurrent-case\n",
    );
    add_v2_fixture_entries(&mut BTreeMap::new()).expect_err("missing v2 fixture must fail");
    write_self_test_file(
        Path::new(V2_MANIFEST),
        "fixture\tcase_id\trequirement\tspec_scenario\nfixtures/v2/control.toml\tcase-current\tCONTROL-001\tcurrent-case\n",
    );
    for (category, id) in [
        (
            "fixture-operation",
            "fixtures/v2/control.toml#case-current#check-current",
        ),
        (
            "fixture-mutation",
            "fixtures/v2/control.toml#case-current#0:first",
        ),
    ] {
        let fixture_key = Key {
            category: category.to_owned(),
            id: id.to_owned(),
        };
        let mut duplicate_fixture = BTreeMap::new();
        duplicate_fixture.insert(
            fixture_key.clone(),
            satisfied(
                category,
                fixture_key.id,
                "duplicate v2 fixture control",
                "point-fixture",
                "duplicate v2 fixture control",
                String::from("sha-256:self-test"),
                "fixtures/v2/control.toml",
            ),
        );
        add_v2_fixture_entries(&mut duplicate_fixture)
            .expect_err("duplicate v2 fixture insertion control must fail");
    }

    write_self_test_file(
        Path::new(V2_EXECUTIONS),
        &format!(
            "{EXECUTION_HEADER}{}",
            self_test_bound_execution_row(
                "fixture-operation",
                "fixtures/v2/control.toml#orphan#orphan",
                "fixtures/v2/control.toml",
                "[[case]]\nid = 'case-current'\noperation = 'check-current'\nmutations = ['first']\n",
            )
        ),
    );
    add_v2_fixture_entries(&mut BTreeMap::new())
        .expect_err("orphan v2 execution evidence must fail");
    write_self_test_file(Path::new(V2_EXECUTIONS), EXECUTION_HEADER);

    fs::remove_dir_all("fixtures/v2/models").expect("v2 model controls must be removable");
    add_models(&mut BTreeMap::new()).expect_err("missing first model root must fail");
    fs::create_dir_all("fixtures/v2/models").expect("v2 model root must be restorable");
    fs::remove_dir_all("fixtures/hardening/v2/models")
        .expect("hardening model controls must be removable");
    add_models(&mut BTreeMap::new()).expect_err("missing second model root must fail");

    write_self_test_file(Path::new("fixtures/bad.tsv"), "first\tsecond\nragged\n");
    add_tsv_fixture_entries(&mut BTreeMap::new()).expect_err("invalid TSV fixture must fail");

    add_platforms(&mut BTreeMap::new()).expect_err("missing platform blockers must fail");

    let platform_root = Path::new(PLATFORM_RESULTS);
    let local_result = platform_root.join(LOCAL_PLATFORM);
    assert!(!platform_result_is_current_at(
        platform_root,
        LOCAL_PLATFORM
    ));
    write_self_test_file(
        &local_result.join("portable-corpus.tsv"),
        &portable_corpus().expect("platform corpus control must render"),
    );
    write_self_test_file(
        &local_result.join("platform-effects.tsv"),
        &platform_effect_diagnostics(LOCAL_PLATFORM).expect("platform effects control must render"),
    );
    let valid_platform_result = format!(
        "schema = 'buoy-platform-result/v1'\nplatform = '{LOCAL_PLATFORM}'\ncommand = 'devenv shell -- buoy-platform-result'\nstatus = 'green'\nprovenance = 'local-host-execution'\nportable_corpus = 'portable-corpus.tsv'\nplatform_effects = 'platform-effects.tsv'\n"
    );
    write_self_test_file(&local_result.join("result.toml"), &valid_platform_result);
    assert!(platform_result_is_current_at(platform_root, LOCAL_PLATFORM));
    for invalid in [
        String::from("not valid platform result TOML = ["),
        valid_platform_result.replacen("buoy-platform-result/v1", "wrong", 1),
        valid_platform_result.replacen(
            &format!("platform = '{LOCAL_PLATFORM}'"),
            "platform = 'invalid'",
            1,
        ),
        valid_platform_result.replacen("devenv shell -- buoy-platform-result", "wrong", 1),
        valid_platform_result.replacen("status = 'green'", "status = 'red'", 1),
        valid_platform_result.replacen("local-host-execution", "registration-only", 1),
        valid_platform_result.replacen("portable-corpus.tsv", "wrong.tsv", 1),
        valid_platform_result.replacen("platform-effects.tsv", "wrong.tsv", 1),
    ] {
        write_self_test_file(&local_result.join("result.toml"), &invalid);
        assert!(!platform_result_is_current_at(
            platform_root,
            LOCAL_PLATFORM
        ));
    }
    write_self_test_file(&local_result.join("result.toml"), &valid_platform_result);
    write_self_test_file(
        Path::new(ADVERSARIAL_SOURCE),
        "// control adversarial source\n",
    );
    write_self_test_file(
        Path::new(PLATFORM_GENERATOR),
        "// control platform generator\n",
    );
    write_self_test_file(Path::new(PLATFORM_BLOCKERS), "control platform blockers\n");
    let mut current_platform_entries = BTreeMap::new();
    add_platforms(&mut current_platform_entries)
        .expect("current local platform result must be accepted");
    assert_eq!(
        current_platform_entries
            .get(&Key {
                category: String::from("platform-backend"),
                id: format!("{LOCAL_PLATFORM}-portable-corpus"),
            })
            .expect("local platform entry must exist")
            .disposition,
        "satisfied"
    );
    let phase_handoffs_control =
        read_text(PHASE_HANDOFFS).expect("phase handoff control must be readable");
    fs::remove_file(PHASE_HANDOFFS).expect("phase handoff control must be removable");
    add_platforms(&mut BTreeMap::new())
        .expect_err("a noncurrent platform requires the local-only phase policy");
    write_self_test_file(Path::new(PHASE_HANDOFFS), &phase_handoffs_control);
    fs::remove_file(ADVERSARIAL_SOURCE).expect("adversarial source control must be removable");
    add_platforms(&mut BTreeMap::new()).expect_err("missing adversarial source digest must fail");
    write_self_test_file(
        Path::new(ADVERSARIAL_SOURCE),
        "// control adversarial source\n",
    );
    write_self_test_file(&local_result.join("portable-corpus.tsv"), "stale corpus\n");
    assert!(!platform_result_is_current_at(
        platform_root,
        LOCAL_PLATFORM
    ));
    let mut stale_platform_entries = BTreeMap::new();
    add_platforms(&mut stale_platform_entries)
        .expect("a stale local result must produce a ledger disposition");
    assert_eq!(
        stale_platform_entries
            .get(&Key {
                category: String::from("platform-backend"),
                id: format!("{LOCAL_PLATFORM}-portable-corpus"),
            })
            .expect("local platform entry must exist")
            .disposition,
        "blocked"
    );
    write_self_test_file(
        &local_result.join("portable-corpus.tsv"),
        &portable_corpus().expect("platform corpus control must restore"),
    );
    write_self_test_file(
        &local_result.join("platform-effects.tsv"),
        "stale effects\n",
    );
    assert!(!platform_result_is_current_at(
        platform_root,
        LOCAL_PLATFORM
    ));
    fs::remove_file(local_result.join("platform-effects.tsv"))
        .expect("platform effects control must be removable");
    assert!(!platform_result_is_current_at(
        platform_root,
        LOCAL_PLATFORM
    ));
    write_self_test_file(
        &local_result.join("platform-effects.tsv"),
        &platform_effect_diagnostics(LOCAL_PLATFORM)
            .expect("platform effects control must restore"),
    );
    fs::remove_file(local_result.join("portable-corpus.tsv"))
        .expect("platform corpus control must be removable");
    assert!(!platform_result_is_current_at(
        platform_root,
        LOCAL_PLATFORM
    ));
    write_self_test_file(
        &platform_root.join("invalid/result.toml"),
        &valid_platform_result.replacen(
            &format!("platform = '{LOCAL_PLATFORM}'"),
            "platform = 'invalid'",
            1,
        ),
    );
    write_self_test_file(
        &platform_root.join("invalid/portable-corpus.tsv"),
        &portable_corpus().expect("invalid platform corpus control must render"),
    );
    write_self_test_file(
        &platform_root.join("invalid/platform-effects.tsv"),
        "invalid effects\n",
    );
    assert!(!platform_result_is_current_at(platform_root, "invalid"));

    write_self_test_file(
        Path::new("openspec/specs/control/spec.md"),
        "### Requirement: CONTROL-001 Synthetic requirement\n#### Scenario: Positive - Current case\n",
    );
    write_self_test_file(
        Path::new(V2_MANIFEST),
        "fixture\tcase_id\trequirement\tspec_scenario\nfixtures/v2/control.toml\tcase-current\tCONTROL-001\tcurrent-case\n",
    );
    write_self_test_file(Path::new(SCENARIO_EXECUTIONS), "");
    add_requirements(&mut BTreeMap::new())
        .expect_err("invalid execution evidence must block requirements");
    write_self_test_file(Path::new(SCENARIO_EXECUTIONS), EXECUTION_HEADER);
    write_self_test_file(
        Path::new("openspec/specs/control/spec.md"),
        "#### Scenario: Positive - Before requirement\n",
    );
    add_requirements(&mut BTreeMap::new()).expect_err("invalid specs must block requirements");
    write_self_test_file(
        Path::new("openspec/specs/control/spec.md"),
        "### Requirement: CONTROL-001 Synthetic requirement\n#### Scenario: Positive - Current case\n",
    );
    let scenario_key = Key {
        category: String::from("scenario"),
        id: String::from("CONTROL-001:current-case"),
    };
    let mut duplicate_scenario = BTreeMap::new();
    duplicate_scenario.insert(
        scenario_key.clone(),
        satisfied(
            "scenario",
            scenario_key.id,
            "duplicate scenario control",
            "point-fixture",
            "duplicate scenario control",
            String::from("sha-256:self-test"),
            "openspec/specs/control/spec.md",
        ),
    );
    add_requirements(&mut duplicate_scenario)
        .expect_err("duplicate scenario insertion control must fail");
    let requirement_key = Key {
        category: String::from("requirement"),
        id: String::from("CONTROL-001"),
    };
    let mut duplicate_requirement = BTreeMap::new();
    duplicate_requirement.insert(
        requirement_key.clone(),
        satisfied(
            "requirement",
            requirement_key.id,
            "duplicate requirement control",
            "point-fixture",
            "duplicate requirement control",
            String::from("sha-256:self-test"),
            "openspec/specs/control/spec.md",
        ),
    );
    add_requirements(&mut duplicate_requirement)
        .expect_err("duplicate requirement insertion control must fail");

    write_self_test_file(
        Path::new(SCENARIO_EXECUTIONS),
        &format!(
            "{EXECUTION_HEADER}{}",
            self_test_bound_execution_row(
                "scenario",
                "CONTROL-ORPHAN:orphan-case",
                "openspec/specs/control/spec.md",
                "### Requirement: CONTROL-001 Synthetic requirement\n#### Scenario: Positive - Current case\n",
            )
        ),
    );
    add_requirements(&mut BTreeMap::new())
        .expect_err("orphan scenario execution evidence must fail");
    write_self_test_file(Path::new(SCENARIO_EXECUTIONS), EXECUTION_HEADER);

    fs::remove_dir_all("fixtures").expect("fixture controls must be removable");
    add_tsv_fixture_entries(&mut BTreeMap::new()).expect_err("missing fixture root must fail");
    write_self_test_file(Path::new(SCHEMA_EXECUTIONS), EXECUTION_HEADER);
    add_tsv_fixture_entries(&mut BTreeMap::new()).expect_err("missing fixture directory must fail");

    for invalid_handoffs in [
        "not valid phase handoff TOML = [",
        "schema = 'missing-phases'\n",
    ] {
        write_self_test_file(Path::new(PHASE_HANDOFFS), invalid_handoffs);
        add_phases(&mut BTreeMap::new()).expect_err("invalid phase handoff control must fail");
    }
    expected_entries().expect_err("incomplete repository control must fail closed");

    std::env::set_current_dir(&repository_root).expect("repository root must be restorable");

    for id in ["linux-portable-corpus", "PLATFORM-WINDOWS-SNAPSHOT-001"] {
        let platform_key = Key {
            category: String::from("platform-backend"),
            id: id.to_owned(),
        };
        let mut duplicate_platform = BTreeMap::new();
        duplicate_platform.insert(
            platform_key.clone(),
            blocked(
                "platform-backend",
                platform_key.id,
                "duplicate platform control",
                "platform-conformance",
                "duplicate platform control",
                String::from("sha-256:self-test"),
                PLATFORM_BLOCKERS,
                PLATFORM_BLOCKERS,
                PLATFORM_BLOCKERS,
            ),
        );
        add_platforms(&mut duplicate_platform)
            .expect_err("duplicate platform insertion control must fail");
    }

    let materialized = sandbox.join("materialized.tsv");
    validate_materialized(&valid, &materialized, Path::new("."), true, false, true)
        .expect("materialized ledger control must pass");
    validate_materialized(&valid, &materialized, Path::new("."), false, false, false)
        .expect("quiet materialized ledger control must pass");
    validate_materialized(
        &blocked_control,
        &materialized,
        Path::new("."),
        true,
        true,
        false,
    )
    .expect_err("green ledger control must reject blocked entries");
    let green = format!(
        "{HEADER}{}\n",
        valid
            .lines()
            .nth(satisfied_index)
            .expect("satisfied row control must exist")
    );
    validate_materialized(
        &green,
        &sandbox.join("green.tsv"),
        Path::new("."),
        true,
        true,
        false,
    )
    .expect("green materialized ledger control must pass");
    fs::write(&materialized, &unsupported).expect("stale ledger control must be writable");
    validate_materialized(&valid, &materialized, Path::new("."), false, false, false)
        .expect_err("stale ledger control must fail");
    validate_materialized(
        &valid,
        &sandbox.join("missing.tsv"),
        Path::new("."),
        false,
        false,
        false,
    )
    .expect_err("missing materialized ledger control must fail");
    validate_materialized(&valid, Path::new("/"), Path::new("."), true, false, false)
        .expect_err("parentless materialized ledger control must fail");
    fs::create_dir_all(sandbox.join("ledger-directory"))
        .expect("ledger directory control must be creatable");
    validate_materialized(
        &valid,
        &sandbox.join("ledger-directory"),
        Path::new("."),
        true,
        false,
        false,
    )
    .expect_err("directory materialized ledger control must fail");
    fs::write(sandbox.join("parent-file"), "not a directory")
        .expect("ledger parent-file control must be writable");
    validate_materialized(
        &valid,
        &sandbox.join("parent-file/ledger.tsv"),
        Path::new("."),
        true,
        false,
        false,
    )
    .expect_err("invalid ledger parent control must fail");
    fs::remove_dir_all(&sandbox).expect("ledger self-test sandbox must be removable");

    let helper_error = expect_diagnostic(Ok(Vec::new()), "CONTROL")
        .expect_err("successful negative control must be rejected");
    expect_diagnostic(Err(helper_error), "LEDGER-NEGATIVE-CONTROL")
        .expect("helper diagnostic control must match");
    expect_diagnostic(Err(String::from("other")), "expected")
        .expect_err("mismatched diagnostic must be rejected");

    println!(
        "assurance-ledger self-test: ok negative-controls={} entries={}",
        negative_control_count,
        rows.len()
    );
}
