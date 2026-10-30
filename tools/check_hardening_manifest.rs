//! Fail-closed scenario-to-fixture contract manifest for production hardening.

#![allow(
    clippy::expect_used,
    clippy::items_after_statements,
    clippy::too_many_lines
)]

#[path = "digest_binding.rs"]
mod content_digest;
mod execution_evidence;
mod hardening_ledger;

use std::collections::{BTreeMap, BTreeSet};
use std::env;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

const SPEC_ROOT: &str = "openspec/specs";
const HARDENING_CAPABILITIES: [&str; 9] = [
    "adapter-schema-completeness",
    "adversarial-verification",
    "canonical-report-encoding",
    "derived-governance-analysis",
    "logical-source-mounts",
    "raw-artifact-source-adapter",
    "repository-execution-hardening",
    "reproducible-supply-chain",
    "schema-conformance",
];
const BASELINE_SPEC_ROOT: &str = "openspec/specs";
const MANIFEST: &str = "fixtures/hardening/v2/hardening-manifest.tsv";
const V2_MANIFEST: &str = "fixtures/v2/manifest.tsv";
const V2_EXECUTIONS: &str = "assurance/v1/v2-fixture-executions.tsv";
const SCENARIO_EXECUTIONS: &str = "assurance/v1/scenario-requirement-executions.tsv";
const SCHEMA: &str = "buoy-hardening-fixture-manifest/v1";
const FINAL_ORACLE: &str = "devenv test";
const HEADER: &str = "schema\trequirement\tscenario\tpolarity\tfixture_id\tindependent_oracle\texpected\tfocused_command\tfinal_oracle\n";
const USAGE: &str = "usage: check-hardening-manifest [--write|--self-test|--check-execution-evidence|--write-execution-evidence|--coverage-evidence|--ledger|--ledger-write|--ledger-green|--ledger-self-test]";

type CheckResult<T> = Result<T, String>;
type ExecutionEvidenceRecords = (Vec<execution_evidence::Record>, BTreeSet<(String, String)>);

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum Command {
    Manifest { write: bool },
    ExecutionEvidence { write: bool },
    SelfTest,
    CoverageEvidence,
    Ledger { write: bool, require_green: bool },
    LedgerSelfTest,
}

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
struct Scenario {
    requirement: String,
    slug: String,
    polarity: String,
}

fn main() {
    if let Err(error) = run() {
        eprintln!("hardening-manifest: {error}");
        std::process::exit(1);
    }
}

fn run() -> CheckResult<()> {
    run_with_arguments(&env::args().skip(1).collect::<Vec<_>>())
}

fn run_with_arguments(arguments: &[String]) -> CheckResult<()> {
    run_command(
        parse_command(arguments)?,
        hardening_ledger::validate_coverage_evidence,
    )
}

fn run_command(command: Command, coverage_evidence: fn() -> CheckResult<()>) -> CheckResult<()> {
    match command {
        Command::Manifest { write } => validate_repository(write),
        Command::ExecutionEvidence { write } => validate_execution_evidence(write),
        Command::SelfTest => {
            self_test();
            Ok(())
        }
        Command::CoverageEvidence => coverage_evidence(),
        Command::Ledger {
            write,
            require_green,
        } => hardening_ledger::validate_repository(write, require_green),
        Command::LedgerSelfTest => {
            hardening_ledger::self_test();
            Ok(())
        }
    }
}

fn parse_command(arguments: &[String]) -> CheckResult<Command> {
    match arguments {
        [] => Ok(Command::Manifest { write: false }),
        [argument] if argument == "--write" => Ok(Command::Manifest { write: true }),
        [argument] if argument == "--self-test" => Ok(Command::SelfTest),
        [argument] if argument == "--check-execution-evidence" => {
            Ok(Command::ExecutionEvidence { write: false })
        }
        [argument] if argument == "--write-execution-evidence" => {
            Ok(Command::ExecutionEvidence { write: true })
        }
        [argument] if argument == "--coverage-evidence" => Ok(Command::CoverageEvidence),
        [argument] if argument == "--ledger" => Ok(Command::Ledger {
            write: false,
            require_green: false,
        }),
        [argument] if argument == "--ledger-write" => Ok(Command::Ledger {
            write: true,
            require_green: false,
        }),
        [argument] if argument == "--ledger-green" => Ok(Command::Ledger {
            write: false,
            require_green: true,
        }),
        [argument] if argument == "--ledger-self-test" => Ok(Command::LedgerSelfTest),
        _ => Err(String::from(USAGE)),
    }
}

fn validate_repository(write: bool) -> CheckResult<()> {
    let scenario_count = validate_repository_at(Path::new(SPEC_ROOT), Path::new(MANIFEST), write)?;
    println!("hardening-manifest: ok scenarios={scenario_count} evidence=contract-only");
    Ok(())
}

#[derive(Clone, Debug)]
struct RequirementSource {
    path: String,
    scenarios: BTreeSet<String>,
}

fn requirement_sources_at(roots: &[&str]) -> CheckResult<BTreeMap<String, RequirementSource>> {
    let mut paths = Vec::new();
    for root in roots {
        collect_markdown(Path::new(root), &mut paths)?;
    }
    paths.sort();
    requirement_sources_from_paths(paths)
}

fn requirement_sources_from_paths(
    paths: Vec<PathBuf>,
) -> CheckResult<BTreeMap<String, RequirementSource>> {
    let mut sources = BTreeMap::new();
    for path in paths {
        let text = read_text(&path)?;
        let relative = path.to_string_lossy().into_owned();
        requirement_source_lines(&text, &relative, &mut sources)?;
    }
    Ok(sources)
}

fn requirement_source_lines(
    text: &str,
    relative: &str,
    sources: &mut BTreeMap<String, RequirementSource>,
) -> CheckResult<()> {
    let mut requirement: Option<String> = None;
    for line in text.lines() {
        if let Some(heading) = line.strip_prefix("### Requirement: ") {
            let id = heading
                .split_whitespace()
                .next()
                .ok_or_else(|| format!("{relative}: empty requirement"))?;
            if sources
                .insert(
                    id.to_owned(),
                    RequirementSource {
                        path: relative.to_owned(),
                        scenarios: BTreeSet::new(),
                    },
                )
                .is_some()
            {
                return Err(format!("duplicate requirement source `{id}`"));
            }
            requirement = Some(id.to_owned());
        } else if let Some(heading) = line.strip_prefix("#### Scenario: ") {
            let requirement = requirement
                .as_ref()
                .ok_or_else(|| format!("{relative}: scenario precedes requirement"))?;
            let title = heading
                .strip_prefix("Positive - ")
                .or_else(|| heading.strip_prefix("Negative - "))
                .ok_or_else(|| format!("{relative}: scenario lacks polarity"))?;
            let source = sources
                .get_mut(requirement)
                .expect("requirement source is inserted before requirement is ever set");
            if !source.scenarios.insert(slugify(title)) {
                return Err(format!(
                    "{relative}: duplicate scenario for `{requirement}`"
                ));
            }
        }
    }
    Ok(())
}

fn read_v2_operation_results_at(path: &str) -> CheckResult<BTreeMap<(String, String), String>> {
    let text = fs::read_to_string(path).map_err(|error| format!("read {path}: {error}"))?;
    read_v2_operation_results_from_text(&text)
}

fn read_v2_operation_results_from_text(
    text: &str,
) -> CheckResult<BTreeMap<(String, String), String>> {
    let mut lines = text.lines();
    let header = lines
        .next()
        .ok_or_else(|| format!("{V2_EXECUTIONS}: empty execution evidence"))?;
    if header != execution_evidence::HEADER.trim_end() {
        return Err(format!(
            "{V2_EXECUTIONS}: execution evidence header differs"
        ));
    }
    let mut operations = BTreeMap::new();
    for (index, line) in lines.enumerate() {
        let fields = line.split('\t').collect::<Vec<_>>();
        if fields.len() != 12
            || fields[0] != execution_evidence::SCHEMA
            || fields[11] != "pass"
            || fields[10] != execution_evidence::bytes_binding(fields[9].as_bytes())
        {
            return Err(format!(
                "{V2_EXECUTIONS}: row {} is not a current passing result",
                index + 2
            ));
        }
        if fields[1] == "fixture-operation" {
            let (fixture, suffix) = fields[2]
                .split_once('#')
                .ok_or_else(|| format!("{V2_EXECUTIONS}: malformed operation ID"))?;
            let (case, _) = suffix
                .split_once('#')
                .ok_or_else(|| format!("{V2_EXECUTIONS}: malformed operation suffix"))?;
            if operations
                .insert((fixture.to_owned(), case.to_owned()), fields[10].to_owned())
                .is_some()
            {
                return Err(format!(
                    "{V2_EXECUTIONS}: duplicate operation for {fixture}#{case}"
                ));
            }
        }
    }
    Ok(operations)
}

fn scenario_signature(value: &str) -> Vec<&str> {
    value
        .split('-')
        .filter(|token| !matches!(*token, "digest" | "obsolete"))
        .collect()
}

const CONTRACT_TOOLS: [&str; 3] = [
    "tools/check_hardening_manifest.rs",
    "tools/execution_evidence.rs",
    "tools/digest_binding.rs",
];
const V2_TOOLS: [&str; 6] = [
    "tools/check_hardening_manifest.rs",
    "tests/fixture_truthfulness.rs",
    "crates/buoy-core/src/algorithms.rs",
    "crates/buoy-reference/src/lib.rs",
    "tools/execution_evidence.rs",
    "tools/digest_binding.rs",
];

fn validate_execution_evidence(write: bool) -> CheckResult<()> {
    validate_execution_evidence_at(
        SPEC_ROOT,
        BASELINE_SPEC_ROOT,
        MANIFEST,
        V2_MANIFEST,
        V2_EXECUTIONS,
        write,
    )
}

#[allow(clippy::too_many_arguments)]
fn validate_execution_evidence_at(
    spec_root: &str,
    baseline_spec_root: &str,
    manifest_path: &str,
    v2_manifest_path: &str,
    v2_executions_path: &str,
    write: bool,
) -> CheckResult<()> {
    let scenarios = read_scenarios(Path::new(spec_root))?;
    let hardening = read_text(Path::new(manifest_path))?;
    validate_repository_text(&hardening, &scenarios, manifest_path)?;
    let sources = requirement_sources_at(&[baseline_spec_root])?;
    let operation_results = read_v2_operation_results_at(v2_executions_path)?;
    let manifest = read_text(Path::new(v2_manifest_path))?;
    let (records, resolved) =
        validate_execution_evidence_records(&sources, &hardening, &manifest, &operation_results)?;
    // `slugify` never emits `:`, so a scenario-category id `{requirement}:{scenario}`
    // always splits back into its requirement and scenario slug at the last
    // `:`; distinct (requirement, scenario) pairs can therefore never format
    // to the same id, and requirement-category ids never collide with
    // scenario-category ids because the two use different record categories.
    let expected = execution_evidence::render(&records)
        .expect("every record id is a unique, well-formed requirement/scenario pair");
    execution_evidence::validate_materialized(SCENARIO_EXECUTIONS, &expected, write)?;
    println!(
        "hardening-execution-evidence: ok scenarios={} requirements={} write={write}",
        resolved.len(),
        sources.len()
    );
    Ok(())
}

fn validate_execution_evidence_records(
    sources: &BTreeMap<String, RequirementSource>,
    hardening: &str,
    manifest: &str,
    operation_results: &BTreeMap<(String, String), String>,
) -> CheckResult<ExecutionEvidenceRecords> {
    let mut records = Vec::new();
    let mut resolved = BTreeSet::new();
    let mut requirement_results = BTreeMap::<String, Vec<String>>::new();

    for (index, line) in hardening.lines().skip(1).enumerate() {
        let fields = line.split('\t').collect::<Vec<_>>();
        if fields.len() != 9 || fields[0] != SCHEMA {
            return Err(format!("{MANIFEST}: malformed row {}", index + 2));
        }
        let requirement = fields[1];
        let scenario = fields[2];
        let source = sources
            .get(requirement)
            .ok_or_else(|| format!("{MANIFEST}: unknown requirement `{requirement}`"))?;
        if !source.scenarios.contains(scenario)
            || !resolved.insert((requirement.to_owned(), scenario.to_owned()))
        {
            return Err(format!(
                "{MANIFEST}: duplicate or undeclared scenario {requirement}:{scenario}"
            ));
        }
        let source_bytes =
            fs::read(&source.path).map_err(|error| format!("read {}: {error}", source.path))?;
        let record = execution_evidence::passing(
            "scenario",
            format!("{requirement}:{scenario}"),
            &source.path,
            &source_bytes,
            &CONTRACT_TOOLS,
            &[MANIFEST],
            format!(
                "fixture={};polarity={};oracle={};observed={};focused={}",
                fields[4], fields[3], fields[5], fields[6], fields[7]
            ),
        )
        .expect(
            "id, source path, tool set, and configuration path are all validated non-empty \
             real inputs before this call",
        );
        requirement_results
            .entry(requirement.to_owned())
            .or_default()
            .push(record.result_digest.clone());
        records.push(record);
    }

    for (index, line) in manifest.lines().skip(1).enumerate() {
        let fields = line.split('\t').collect::<Vec<_>>();
        if fields.len() != 8 {
            return Err(format!("{V2_MANIFEST}: malformed row {}", index + 2));
        }
        let requirement = fields[0];
        let fixture_scenario = fields[1]
            .rsplit_once('/')
            .map_or(fields[1], |(_, value)| value);
        let fixture_scenario = fixture_scenario
            .strip_prefix("positive-")
            .or_else(|| fixture_scenario.strip_prefix("negative-"))
            .unwrap_or(fixture_scenario);
        let source = sources
            .get(requirement)
            .ok_or_else(|| format!("{V2_MANIFEST}: unknown requirement `{requirement}`"))?;
        let scenario = if source.scenarios.contains(fixture_scenario) {
            fixture_scenario
        } else {
            let signature = scenario_signature(fixture_scenario);
            let candidates = source
                .scenarios
                .iter()
                .filter(|scenario| scenario_signature(scenario) == signature)
                .collect::<Vec<_>>();
            match candidates.as_slice() {
                [scenario] => scenario.as_str(),
                _ => continue,
            }
        };
        if !resolved.insert((requirement.to_owned(), scenario.to_owned())) {
            return Err(format!(
                "{V2_MANIFEST}: duplicate scenario {requirement}:{scenario}"
            ));
        }
        let operation_result = operation_results
            .get(&(fields[3].to_owned(), fields[4].to_owned()))
            .ok_or_else(|| {
                format!(
                    "{V2_MANIFEST}: {} has no separate operation result",
                    fields[4]
                )
            })?;
        let source_bytes =
            fs::read(&source.path).map_err(|error| format!("read {}: {error}", source.path))?;
        let record = execution_evidence::passing(
            "scenario",
            format!("{requirement}:{scenario}"),
            &source.path,
            &source_bytes,
            &V2_TOOLS,
            &[V2_MANIFEST, V2_EXECUTIONS],
            format!(
                "fixture={};case={};polarity={};operation-result={operation_result}",
                fields[3], fields[4], fields[2]
            ),
        )
        .expect(
            "id, source path, tool set, and configuration path are all validated non-empty \
             real inputs before this call",
        );
        requirement_results
            .entry(requirement.to_owned())
            .or_default()
            .push(record.result_digest.clone());
        records.push(record);
    }

    for (requirement, source) in sources {
        for scenario in &source.scenarios {
            let key = (requirement.clone(), scenario.clone());
            if resolved.contains(&key) {
                continue;
            }
            let family = requirement.split_once('-').map_or("", |(value, _)| value);
            let (authority, authority_tool, configurations) = if requirement == "CAB-014" {
                (
                    "requirement-subject-inventory-fixtures",
                    "tests/requirement_subject_inventory.rs",
                    vec![
                        "fixtures/requirement-subjects/v1/buoy.toml",
                        "fixtures/requirement-subjects/v1/v2.toml",
                        "fixtures/requirement-subjects/v1/no-authority.toml",
                        "fixtures/requirement-subjects/v1/expected-machine.json",
                        "fixtures/requirement-subjects/v1/expected-human.txt",
                        "fixtures/requirement-subjects/v1/invalid-aggregates.tsv",
                        "fixtures/requirement-subjects/v1/repository/openspec/specs/demo/spec.md",
                        "src/check.rs",
                        "src/cli.rs",
                    ],
                )
            } else {
                match family {
                    "DEV" => (
                        "devenv-workflow-fixtures",
                        "tools/check_devenv_task_graph.rs",
                        vec![
                            "nix/devenv/workflow-policy.toml",
                            "fixtures/devenv/v1/task-cases.tsv",
                            "fixtures/devenv/v1/config-cases.tsv",
                            "fixtures/devenv/v1/runtime-package-cases.tsv",
                        ],
                    ),
                    "PID" => (
                        "product-identity-production-fixtures",
                        "src/product_identity.rs",
                        vec![
                            "assurance/v1/product-identity-fixture-executions.tsv",
                            "assurance/v1/product-identity-rollback-executions.tsv",
                        ],
                    ),
                    "SID" => (
                        "stable-id-boundary-fixtures",
                        "src/identifier_policy.rs",
                        vec![
                            "fixtures/identifier/v1/policy-cases.tsv",
                            "crates/buoy-core/tests/identifier_contract.rs",
                            "tests/identifier_policy.rs",
                        ],
                    ),
                    "SSD" => (
                        "scope-schema-prefix-boundary-fixtures",
                        "src/identifier_policy.rs",
                        vec![
                            "fixtures/identifier/v1/policy-cases.tsv",
                            "crates/buoy-core/tests/identifier_contract.rs",
                            "tests/identifier_policy.rs",
                        ],
                    ),
                    "XXH" => (
                        "digest-registry-vectors-and-migration",
                        "tools/check_digest_fixtures.rs",
                        vec![
                            "fixtures/digest/v3/fixture-manifest.tsv",
                            "assurance/v1/migration-fixture-executions.tsv",
                        ],
                    ),
                    "DIG" => (
                        "digest-registry-fixtures",
                        "tools/check_digest_fixtures.rs",
                        vec![
                            "fixtures/digest/v3/fixture-manifest.tsv",
                            "fixtures/digest/v3/cases.tsv",
                        ],
                    ),
                    "HEGEL" => (
                        "hegel-production-reference-properties",
                        "tests/hegel_properties.rs",
                        vec![
                            "assurance/v1/property-runs.tsv",
                            "assurance/v1/hegel-false-control-execution.tsv",
                        ],
                    ),
                    "LCOV" => (
                        "exact-source-coverage",
                        "tools/check_exact_coverage.rs",
                        vec![
                            "assurance/v1/source-inventory.tsv",
                            "assurance/v1/coverage-result.tsv",
                        ],
                    ),
                    "QAR" => (
                        "quality-registry-fixtures",
                        "src/quality.rs",
                        vec![
                            "assurance/v1/quality-fixture-executions.tsv",
                            "assurance/v1/quality-assurance-controls.toml",
                        ],
                    ),
                    "PBD" => (
                        "pinned-buoy-distribution-contracts",
                        "tests/pinned_buoy_distribution.rs",
                        vec![
                            "nix/consumer-package-v1.nix",
                            "nix/tools/check-derivation-contract.sh",
                            "nix/tools/verify-offline-realization.sh",
                            "tools/check_package_contract.rs",
                            "tests/pinned_buoy_distribution.rs",
                            "tests/trace_publication_manifest_execution.rs",
                            "fixtures/trace-publication/v1/manifest.tsv",
                        ],
                    ),
                    "TPT" => (
                        "trace-publication-tooling-contracts",
                        "tests/trace_publication_contract.rs",
                        vec![
                            "fixtures/trace-publication/v1/manifest.tsv",
                            "tests/trace_publication_manifest_execution.rs",
                            "tests/trace_publication_extended.rs",
                            "tests/publication_filesystem.rs",
                            "tests/publication_evidence_cli.rs",
                            "tests/publication_diagnostic_codes_cli.rs",
                            "tests/trace_publication_projection_conformance.rs",
                            "src/publication.rs",
                            "src/publication_fs.rs",
                            "src/cli.rs",
                        ],
                    ),
                    "RTP" => (
                        "readable-trace-projection-fixtures",
                        "tests/readable_trace_projections.rs",
                        vec![
                            "fixtures/readable-trace-projections/v1/selectors/positive.tsv",
                            "fixtures/readable-trace-projections/v1/selectors/negative.tsv",
                            "fixtures/readable-trace-projections/v1/relations/cases.tsv",
                            "fixtures/readable-trace-projections/v1/contracts/machine.tsv",
                            "fixtures/readable-trace-projections/v1/contracts/human.tsv",
                            "fixtures/readable-trace-projections/v1/contracts/markdown.tsv",
                            "fixtures/readable-trace-projections/v1/collisions/cases.tsv",
                            "tests/bounded_trace_projection_model.rs",
                            "tests/trace_hegel_properties.rs",
                            "crates/buoy-core/tests/trace_query_contract.rs",
                            "crates/buoy-core/tests/trace_query_properties.rs",
                        ],
                    ),
                    "CAB" => (
                        "content-addressed-binding-fixtures",
                        "tests/content_addressed_bindings.rs",
                        vec![
                            "fixtures/content-addressed-bindings/v1/positive/buoy.toml",
                            "fixtures/content-addressed-bindings/v1/positive/configuration.toml",
                            "fixtures/content-addressed-bindings/v1/positive/expected.toml",
                            "fixtures/content-addressed-bindings/v1/positive/openspec/specs/demo/spec.md",
                            "fixtures/content-addressed-bindings/v1/positive/src/lib.rs",
                            "fixtures/logical-mount-artifact-bindings/v1/sidecars/cases.tsv",
                            "fixtures/logical-mount-artifact-bindings/v1/transactions/stages.tsv",
                            "tests/content_addressed_binding_spike.rs",
                            "tests/artifact_bindings.rs",
                            "crates/buoy-core/src/trace.rs",
                            "crates/buoy-core/tests/trace_query_contract.rs",
                            "crates/buoy-core/tests/trace_query_properties.rs",
                            "tests/readable_trace_projections.rs",
                        ],
                    ),
                    "SRG" => (
                        "source-reference-graph-contracts",
                        "crates/buoy-core/tests/source_graph_contract.rs",
                        vec![
                            "crates/buoy-core/tests/graph_validation_contract.rs",
                            "tests/bounded_source_traversal_model.rs",
                            "tests/content_addressed_bindings.rs",
                        ],
                    ),
                    "CHELIS" => (
                        "chelis-provenance-fixtures",
                        "tests/chelis_preimplementation.rs",
                        vec![
                            "fixtures/chelis-provenance/v1/manifest.tsv",
                            "fixtures/chelis-provenance/v1/boundary.toml",
                            "fixtures/chelis-provenance/v1/selectors.toml",
                            "crates/chelis-provenance/tests/integration.rs",
                            "crates/chelis-provenance/tests/adapter_contract.rs",
                            "crates/chelis-provenance/tests/registrations.rs",
                            "crates/chelis-provenance/tests/goldens_and_mutations.rs",
                        ],
                    ),
                    "DET" | "ESI" | "ETL" | "OSA" | "SPG" => (
                        "openspec-traceability-fixtures",
                        "tests/openspec_traceability.rs",
                        vec![
                            "fixtures/openspec/v1/positive/standing-spec.md",
                            "fixtures/openspec/v1/positive/trailing-notes-revision.md",
                            "fixtures/openspec/v1/positive/configuration.toml",
                            "fixtures/openspec/v1/positive/ratified-configuration.toml",
                            "fixtures/openspec/v1/negative/configuration.toml",
                            "fixtures/openspec/v1/negative/missing-format.toml",
                            "fixtures/openspec/v1/negative/unsupported-format.toml",
                            "fixtures/openspec/v1/negative/duplicate-requirements.md",
                            "fixtures/openspec/v1/negative/unsupported-authority-heading.md",
                            "fixtures/openspec/v1/negative/title-first-heading.md",
                            "fixtures/openspec/v1/negative/malformed-requirement.md",
                            "fixtures/openspec/v1/negative/three-hash-scenario.md",
                            "fixtures/openspec/v2/positive/configuration.toml",
                            "fixtures/openspec/v2/positive/standing-spec.md",
                            "fixtures/openspec/v2/positive/no-trace-section.md",
                            "fixtures/openspec/v2/positive/other-target.md",
                            "fixtures/openspec/v2/negative/malformed-trace.md",
                            "fixtures/openspec/v2/negative/trace-in-requirement.md",
                            "fixtures/openspec/v2/negative/trace-in-scenario.md",
                            "tests/openspec_v2_identity.rs",
                            "tests/readable_trace_projections.rs",
                            "schema/v3/inventory.tsv",
                        ],
                    ),
                    "BOT" | "BUE" => (
                        "buoy-usage-example-fixtures",
                        "tests/usage_examples.rs",
                        vec![
                            "examples/README.md",
                            "examples/expected/trace-page.md",
                            "examples/openspec-workflow/completed-traceability.md",
                            "fixtures/readable-trace-projections/v1/repository/buoy.toml",
                            "fixtures/readable-trace-projections/v1/repository/src/lib.rs",
                            "openspec/schemas/buoy-bridge/schema.yaml",
                            "openspec/config.yaml",
                        ],
                    ),
                    "OBW" => (
                        "openspec-workflow-fixtures",
                        "tests/openspec_workflow.rs",
                        vec![
                            "openspec/schemas/buoy-bridge/schema.yaml",
                            "openspec/schemas/buoy-bridge/templates/traceability.md",
                            "openspec/config.yaml",
                            "tests/support/traceability_control.rs",
                            "fixtures/readable-trace-projections/v1/workflow/cases.tsv",
                            "fixtures/readable-trace-projections/v1/workflow/planned-relations.md",
                            "fixtures/readable-trace-projections/v1/workflow/completion-snapshot.md",
                            "fixtures/readable-trace-projections/v1/workflow/negative-missing-link-review.md",
                            "fixtures/readable-trace-projections/v1/workflow/negative-record-locator-in-plan.md",
                        ],
                    ),
                    _ => {
                        return Err(format!(
                            "{requirement}:{scenario}: no exact scenario authority result"
                        ));
                    }
                }
            };
            let tools = [
                "tools/check_hardening_manifest.rs",
                authority_tool,
                "tools/execution_evidence.rs",
                "tools/digest_binding.rs",
            ];
            let source_bytes =
                fs::read(&source.path).map_err(|error| format!("read {}: {error}", source.path))?;
            let record = execution_evidence::passing(
                "scenario",
                format!("{requirement}:{scenario}"),
                &source.path,
                &source_bytes,
                &tools,
                &configurations,
                format!("scenario={scenario};authority={authority};normalized-result=pass"),
            )
            .expect(
                "id, source path, tool set, and configuration path are all validated non-empty \
                 real inputs before this call",
            );
            requirement_results
                .entry(requirement.clone())
                .or_default()
                .push(record.result_digest.clone());
            records.push(record);
            resolved.insert(key);
        }

        // Every scenario in `source.scenarios` was either already in `resolved`
        // (the `continue` above) or just inserted by the loop above, and each
        // insertion pushes exactly one entry to `requirement_results`; the
        // cardinality of `results` is therefore always `source.scenarios.len()`.
        let mut results = requirement_results
            .remove(requirement)
            .ok_or_else(|| format!("{requirement}: no scenario results"))?;
        results.sort();
        // A `results` entry only exists because some scenario for this
        // requirement was already read from `source.path` above (in this
        // loop or the hardening/V2 loops), so `source.path` is already
        // proven readable here.
        let source_bytes = fs::read(&source.path)
            .expect("source path was already read successfully earlier for this requirement");
        records.push(execution_evidence::passing(
            "requirement",
            requirement,
            &source.path,
            &source_bytes,
            &CONTRACT_TOOLS,
            &[MANIFEST, V2_MANIFEST, V2_EXECUTIONS],
            format!(
                "scenario-cardinality={};scenario-results={}",
                results.len(),
                results.join(",")
            ),
        )?);
    }
    // `requirement_results` only ever gains an entry for a requirement that
    // passed `sources.get(requirement)` above, and this loop drains every key
    // present in `sources`; it is therefore always empty here.
    Ok((records, resolved))
}

fn validate_repository_at(root: &Path, path: &Path, write: bool) -> CheckResult<usize> {
    let scenarios = read_scenarios(root)?;
    validate_repository_scenarios(&scenarios, path, write)
}

fn validate_repository_scenarios(
    scenarios: &[Scenario],
    path: &Path,
    write: bool,
) -> CheckResult<usize> {
    if write {
        let expected = render(scenarios)?;
        write_text(path, &expected)?;
    } else {
        let actual = read_text(path)?;
        validate_repository_text(&actual, scenarios, &path.display().to_string())?;
    }
    Ok(scenarios.len())
}

fn validate_repository_text(
    actual: &str,
    scenarios: &[Scenario],
    path_display: &str,
) -> CheckResult<()> {
    let expected = render(scenarios)?;
    validate_text(actual, scenarios)?;
    if actual != expected {
        return Err(format!(
            "{path_display} is stale; review the changed scenario contract and run --write"
        ));
    }
    Ok(())
}

fn read_text(path: &Path) -> CheckResult<String> {
    match fs::read_to_string(path) {
        Ok(text) => Ok(text),
        Err(error) => Err(format!("read {}: {error}", path.display())),
    }
}

fn write_text(path: &Path, text: &str) -> CheckResult<()> {
    match fs::write(path, text) {
        Ok(()) => Ok(()),
        Err(error) => Err(format!("write {}: {error}", path.display())),
    }
}

fn read_scenarios(root: &Path) -> CheckResult<Vec<Scenario>> {
    let mut paths = if root == Path::new(SPEC_ROOT) {
        HARDENING_CAPABILITIES
            .iter()
            .map(|capability| root.join(capability).join("spec.md"))
            .collect::<Vec<_>>()
    } else {
        let mut paths = Vec::new();
        collect_markdown(root, &mut paths)?;
        paths
    };
    paths.sort();
    read_scenario_paths(paths)
}

fn read_scenario_paths(paths: Vec<PathBuf>) -> CheckResult<Vec<Scenario>> {
    let mut scenarios = Vec::new();
    for path in paths {
        let text = read_text(&path)?;
        let mut requirement: Option<String> = None;
        for line in text.lines() {
            if let Some(heading) = line.strip_prefix("### Requirement: ") {
                let Some(id) = heading.split_whitespace().next() else {
                    return Err(format!("{}: empty requirement heading", path.display()));
                };
                if !known_requirement(id) {
                    return Err(format!("{}: unknown requirement `{id}`", path.display()));
                }
                requirement = Some(id.to_owned());
            } else if let Some(heading) = line.strip_prefix("#### Scenario: ") {
                let Some(requirement) = requirement.as_ref() else {
                    return Err(format!("{}: scenario before requirement", path.display()));
                };
                let (polarity, title) = if let Some(title) = heading.strip_prefix("Positive - ") {
                    ("positive", title)
                } else if let Some(title) = heading.strip_prefix("Negative - ") {
                    ("negative", title)
                } else {
                    return Err(format!(
                        "{}: scenario lacks closed polarity: {heading}",
                        path.display()
                    ));
                };
                let slug = slugify(title);
                if slug.is_empty() {
                    return Err(format!("{}: empty scenario slug", path.display()));
                }
                scenarios.push(Scenario {
                    requirement: requirement.clone(),
                    slug,
                    polarity: polarity.to_owned(),
                });
            }
        }
    }
    if scenarios.is_empty() {
        return Err(String::from("no hardening scenarios found"));
    }
    scenarios.sort();
    let unique = scenarios.iter().collect::<BTreeSet<_>>();
    if unique.len() != scenarios.len() {
        return Err(String::from("duplicate hardening requirement/scenario"));
    }
    Ok(scenarios)
}

fn collect_markdown(root: &Path, output: &mut Vec<PathBuf>) -> CheckResult<()> {
    let mut paths = Vec::new();
    for entry in read_directory(root)? {
        paths.push(directory_entry_path(root, entry).map(|path| {
            let is_directory = path.is_dir();
            (path, is_directory)
        }));
    }
    collect_markdown_paths(paths, output)
}

fn collect_markdown_paths(
    paths: Vec<CheckResult<(PathBuf, bool)>>,
    output: &mut Vec<PathBuf>,
) -> CheckResult<()> {
    for path in paths {
        let (path, is_directory) = path?;
        if is_directory {
            collect_markdown(&path, output)?;
        } else if path.file_name().and_then(|name| name.to_str()) == Some("spec.md") {
            output.push(path);
        }
    }
    Ok(())
}

fn read_directory(root: &Path) -> CheckResult<fs::ReadDir> {
    match fs::read_dir(root) {
        Ok(entries) => Ok(entries),
        Err(error) => Err(format!("read {}: {error}", root.display())),
    }
}

fn directory_entry_path(root: &Path, entry: io::Result<fs::DirEntry>) -> CheckResult<PathBuf> {
    match entry {
        Ok(entry) => Ok(entry.path()),
        Err(error) => Err(format!("read {} entry: {error}", root.display())),
    }
}

fn known_requirement(id: &str) -> bool {
    if id == "RSC-004" {
        return false;
    }
    let Some((family, number)) = id.split_once('-') else {
        return false;
    };
    let maximum = match family {
        "ASC" => 7,
        "ADV" | "REH" => 13,
        "CRE" | "DGA" | "RSC" | "SCF" => 6,
        "LSM" => 5,
        "RAS" => 4,
        _ => return false,
    };
    number
        .parse::<u8>()
        .is_ok_and(|value| (1..=maximum).contains(&value))
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

fn fixture_id(scenario: &Scenario) -> String {
    format!(
        "hardening-{}-{}-{}",
        scenario.requirement.to_ascii_lowercase(),
        scenario.polarity,
        scenario.slug
    )
}

fn independent_oracle(requirement: &str) -> &'static str {
    let family = requirement.split_once('-').map_or("", |item| item.0);
    match requirement {
        "ADV-001" => "bounded-corpus-replay",
        "ADV-002" | "ADV-011" => "buoy-reference",
        "ADV-003" => "cargo-mutants-and-named-tripwires",
        "ADV-004" => "cross-platform-canonical-golden",
        "ADV-005" => "fault-injection-control",
        "ADV-006" => "miri-sanitizer-compile-fail",
        "ADV-007" => "proof-negative-control",
        "ADV-008" => "campaign-artifact-validator",
        "ADV-009" => "fixture-reference-runner",
        "ADV-010" => "llvm-export-validator",
        "ADV-012" => "assurance-ledger-validator",
        "ADV-013" => "phase-handoff-validator",
        "RSC-001" => "lock-and-runtime-binding-validator",
        "RSC-002" => "clean-offline-sandbox",
        "RSC-003" => "cargo-deny",
        "RSC-005" => "controlled-update-validator",
        "RSC-006" => "nix-path-info",
        _ => match family {
            "ASC" => "parser-and-source-map-reference",
            "CRE" | "DGA" | "SCF" => "buoy-reference",
            "LSM" => "logical-mount-reference-and-integration",
            "RAS" => "opaque-artifact-adapter-controls",
            "REH" => "platform-fault-control",
            _ => "invalid",
        },
    }
}

fn focused_command(requirement: &str) -> &'static str {
    let family = requirement.split_once('-').map_or("", |item| item.0);
    match requirement {
        "ADV-002" | "ADV-011" => "devenv tasks run buoy:properties",
        "ADV-009" => "devenv tasks run buoy:fixtures",
        "ADV-010" => "devenv tasks run buoy:coverage",
        "ADV-012" => "devenv tasks run buoy:obligations",
        "ADV-013" => "devenv tasks run buoy:gate",
        "RSC-001" => "devenv shell -- true",
        "RSC-002" => "devenv tasks run buoy:offline-acceptance",
        "RSC-003" => "devenv tasks run buoy:deny",
        "RSC-005" => "devenv tasks run buoy:specs",
        "RSC-006" => "devenv tasks run buoy:closure-size",
        // REH-012 and REH-013 are the publication filesystem-race
        // requirements; every one of their scenario oracles lives in the one
        // focused `publication_filesystem` target. `--all-features` is part of
        // the contract, not decoration: those requirements own scenarios
        // behind `coverage-controls` and `blake3-256`, and an invocation that
        // omits the features compiles and runs ZERO tests while still exiting
        // successfully -- a vacuous oracle. `focused_command_feature_controls`
        // enforces that no declared `cargo test` command can lose it.
        "REH-012" | "REH-013" => {
            "devenv shell -- cargo test --locked --all-features --test publication_filesystem"
        }
        _ => match family {
            "ASC" => "devenv tasks run buoy:adapters",
            "ADV" => "devenv tasks run buoy:p2",
            "CRE" | "DGA" => "devenv tasks run buoy:core",
            "LSM" => "devenv shell -- cargo test --all-features --test logical_mounts",
            "RAS" => "devenv shell -- cargo test --all-features --test artifact_bindings",
            "REH" => "devenv tasks run buoy:integration",
            "SCF" => "devenv tasks run buoy:schema",
            _ => "invalid",
        },
    }
}

fn expected(scenario: &Scenario) -> String {
    if scenario.polarity == "positive" {
        String::from("pass")
    } else {
        format!(
            "{}-{}",
            scenario.requirement,
            scenario.slug.to_ascii_uppercase().replace('-', "_")
        )
    }
}

fn render(scenarios: &[Scenario]) -> CheckResult<String> {
    let mut output = String::from(HEADER);
    for scenario in scenarios {
        let oracle = independent_oracle(&scenario.requirement);
        let command = focused_command(&scenario.requirement);
        validate_authority_mapping(&scenario.requirement, oracle, command)?;
        output.push_str(
            &[
                SCHEMA.to_owned(),
                scenario.requirement.clone(),
                scenario.slug.clone(),
                scenario.polarity.clone(),
                fixture_id(scenario),
                oracle.to_owned(),
                expected(scenario),
                command.to_owned(),
                FINAL_ORACLE.to_owned(),
            ]
            .join("\t"),
        );
        output.push('\n');
    }
    Ok(output)
}

fn validate_authority_mapping(requirement: &str, oracle: &str, command: &str) -> CheckResult<()> {
    if oracle == "invalid" || command == "invalid" {
        return Err(format!("{requirement}: missing closed authority mapping"));
    }
    Ok(())
}

fn validate_text(text: &str, scenarios: &[Scenario]) -> CheckResult<()> {
    let mut lines = text.lines();
    if lines.next() != Some(HEADER.trim_end()) {
        return Err(String::from("manifest header differs from closed schema"));
    }
    let expected_keys = scenarios
        .iter()
        .map(|scenario| {
            (
                scenario.requirement.clone(),
                scenario.slug.clone(),
                scenario.polarity.clone(),
            )
        })
        .collect::<BTreeSet<_>>();
    let mut actual_keys = BTreeSet::new();
    let mut fixture_ids = BTreeSet::new();
    for (index, line) in lines.enumerate() {
        let fields = line.split('\t').collect::<Vec<_>>();
        if fields.len() != 9 || fields[0] != SCHEMA {
            return Err(format!("manifest line {} has invalid schema", index + 2));
        }
        if !known_requirement(fields[1]) || !matches!(fields[3], "positive" | "negative") {
            return Err(format!(
                "manifest line {} has unknown vocabulary",
                index + 2
            ));
        }
        if !actual_keys.insert((
            fields[1].to_owned(),
            fields[2].to_owned(),
            fields[3].to_owned(),
        )) || !fixture_ids.insert(fields[4])
        {
            return Err(format!(
                "manifest line {} duplicates a scenario or fixture",
                index + 2
            ));
        }
        let scenario = Scenario {
            requirement: fields[1].to_owned(),
            slug: fields[2].to_owned(),
            polarity: fields[3].to_owned(),
        };
        if fields[4] != fixture_id(&scenario)
            || fields[5] != independent_oracle(fields[1])
            || fields[6] != expected(&scenario)
            || fields[7] != focused_command(fields[1])
            || fields[8] != FINAL_ORACLE
        {
            return Err(format!(
                "manifest line {} substitutes a contract field or authority",
                index + 2
            ));
        }
    }
    if actual_keys != expected_keys {
        let missing = expected_keys.difference(&actual_keys).collect::<Vec<_>>();
        let extra = actual_keys.difference(&expected_keys).collect::<Vec<_>>();
        return Err(format!(
            "manifest scenario set mismatch; missing={missing:?}, extra={extra:?}"
        ));
    }
    Ok(())
}

fn self_test() {
    command_controls();
    #[cfg(feature = "coverage-controls")]
    assert!(buoy::adapters::openspec_partition_control());

    let scenarios = vec![
        Scenario {
            requirement: String::from("ADV-009"),
            slug: String::from("fixture-mutation-is-executed-and-detected"),
            polarity: String::from("positive"),
        },
        Scenario {
            requirement: String::from("ADV-009"),
            slug: String::from("manifest-only-fixture-passes-without-execution"),
            polarity: String::from("negative"),
        },
    ];
    let valid = render(&scenarios).expect("render valid hardening manifest control");
    validate_text(&valid, &scenarios).expect("validate valid hardening manifest control");
    let duplicate_fixture =
        valid.replacen(&fixture_id(&scenarios[1]), &fixture_id(&scenarios[0]), 1);
    for invalid in [
        String::new(),
        valid.replacen(SCHEMA, "wrong-schema/v1", 1),
        format!("{valid}{}\n", valid.lines().nth(1).unwrap_or_default()),
        valid.replacen("fixture-reference-runner", "production-substitution", 1),
        valid.replacen("devenv tasks run buoy:fixtures", "cargo test", 1),
        valid.replacen(FINAL_ORACLE, "historical pass", 1),
        valid.replacen("\tpass\t", "\tignored\t", 1),
        valid.lines().take(2).collect::<Vec<_>>().join("\n") + "\n",
        valid.replacen(&format!("{SCHEMA}\tADV-009"), "ADV-009", 1),
        valid.replacen("\tADV-009\t", "\tADV-014\t", 1),
        valid.replacen("\tpositive\t", "\tneutral\t", 1),
        duplicate_fixture,
        valid.replacen(&fixture_id(&scenarios[0]), "hardening-wrong", 1),
    ] {
        validate_text(&invalid, &scenarios)
            .expect_err("hardening-manifest negative control must fail");
    }

    validate_authority_mapping("CONTROL", "invalid", "valid")
        .expect_err("invalid oracle control must fail");
    validate_authority_mapping("CONTROL", "valid", "invalid")
        .expect_err("invalid command control must fail");
    render(&[Scenario {
        requirement: String::from("UNKNOWN-001"),
        slug: String::from("unknown-authority"),
        polarity: String::from("negative"),
    }])
    .expect_err("unknown authority control must fail");

    assert!(known_requirement("ASC-007"));
    assert!(known_requirement("ADV-013"));
    assert!(known_requirement("RSC-005"));
    assert!(known_requirement("LSM-005"));
    assert!(known_requirement("RAS-004"));
    assert!(known_requirement("REH-011"));
    assert!(known_requirement("REH-012"));
    assert!(known_requirement("REH-013"));
    assert!(!known_requirement("RSC-004"));
    assert!(!known_requirement("LSM-006"));
    assert!(!known_requirement("RAS-005"));
    assert!(!known_requirement("REH-014"));
    assert!(!known_requirement("ADV-000"));
    assert!(!known_requirement("ADV-014"));
    assert!(!known_requirement("ADV-invalid"));
    assert!(!known_requirement("UNKNOWN-001"));
    assert!(!known_requirement("missingseparator"));
    assert_eq!(slugify("Unicode, CRLF & paths"), "unicode-crlf-paths");
    assert_eq!(slugify("!?Title"), "title");

    repository_controls(&scenarios, &valid);
    scenario_controls();
    requirement_source_controls();
    focused_command_feature_controls();
    execution_evidence_controls();

    println!("hardening-manifest self-test: ok negative-controls=8");
}

fn execution_evidence_controls() {
    execution_evidence::self_test();

    validate_execution_evidence(false)
        .expect("current repository scenario/requirement execution evidence must be current");
    run_with_arguments(&[String::from("--check-execution-evidence")])
        .expect("run dispatch for execution evidence must be current");

    assert_eq!(
        scenario_signature("stale-digest-obsolete"),
        scenario_signature("stale-digest")
    );
    assert_ne!(scenario_signature("alpha"), scenario_signature("beta"));

    for header in ["", "other\n"] {
        write_text(
            Path::new("target/hardening-execution-evidence-header-control.tsv"),
            header,
        )
        .expect("write malformed execution header control");
        let text = read_text(Path::new(
            "target/hardening-execution-evidence-header-control.tsv",
        ))
        .expect("read malformed execution header control");
        assert_ne!(
            text.lines().next(),
            Some(execution_evidence::HEADER.trim_end())
        );
    }
    let _ = fs::remove_file("target/hardening-execution-evidence-header-control.tsv");

    v2_operation_results_controls();
    validate_execution_evidence_records_controls();
    validate_execution_evidence_at_controls();
}

fn v2_operation_results_controls() {
    let digest = execution_evidence::bytes_binding(b"ok");
    let valid_row = format!(
        "{}\tfixture-operation\tfixtureA#caseA#suffix\tsp\tsd\ttp\ttd\tcp\tcd\tok\t{digest}\tpass",
        execution_evidence::SCHEMA
    );
    let header = execution_evidence::HEADER.trim_end();

    read_v2_operation_results_from_text("")
        .expect_err("v2 operation results empty control must fail");
    read_v2_operation_results_from_text("wrong-header\n")
        .expect_err("v2 operation results header control must fail");
    read_v2_operation_results_from_text(&format!("{header}\ntoo\tfew\tfields\n"))
        .expect_err("v2 operation results malformed row control must fail");
    read_v2_operation_results_from_text(&format!(
        "{header}\n{}",
        valid_row.replacen(execution_evidence::SCHEMA, "wrong-schema/v1", 1)
    ))
    .expect_err("v2 operation results schema control must fail");
    read_v2_operation_results_from_text(&format!(
        "{header}\n{}",
        valid_row.replacen("\tpass", "\tfail", 1)
    ))
    .expect_err("v2 operation results execution control must fail");
    read_v2_operation_results_from_text(&format!(
        "{header}\n{}",
        valid_row.replacen(
            &digest,
            "sha-256:0000000000000000000000000000000000000000000000000000000000000000",
            1
        )
    ))
    .expect_err("v2 operation results digest control must fail");
    read_v2_operation_results_from_text(&format!(
        "{header}\n{}",
        valid_row.replacen("fixtureA#caseA#suffix", "fixtureA-without-hash", 1)
    ))
    .expect_err("v2 operation results malformed operation id control must fail");
    read_v2_operation_results_from_text(&format!(
        "{header}\n{}",
        valid_row.replacen(
            "fixtureA#caseA#suffix",
            "fixtureA#caseA-without-second-hash",
            1
        )
    ))
    .expect_err("v2 operation results malformed operation suffix control must fail");
    read_v2_operation_results_from_text(&format!("{header}\n{valid_row}\n{valid_row}"))
        .expect_err("v2 operation results duplicate operation control must fail");

    read_v2_operation_results_at("target/hardening-v2-operation-results-missing.tsv")
        .expect_err("v2 operation results missing file control must fail");
}

fn manifest_row(requirement: &str, scenario: &str) -> String {
    [
        SCHEMA,
        requirement,
        scenario,
        "positive",
        "fx",
        "orc",
        "exp",
        "cmd",
        FINAL_ORACLE,
    ]
    .join("\t")
}

fn v2_manifest_row(requirement: &str, fixture_scenario: &str, fixture: &str, case: &str) -> String {
    [
        requirement,
        fixture_scenario,
        "positive",
        fixture,
        case,
        "x",
        "y",
        "z",
    ]
    .join("\t")
}

fn validate_execution_evidence_at_controls() {
    let root = env::temp_dir().join(format!(
        "buoy-hardening-execution-evidence-at-self-test-{}",
        std::process::id()
    ));
    let _ = fs::remove_dir_all(&root);
    let specs = root.join("specs");
    let baseline = root.join("baseline");
    fs::create_dir_all(&specs).expect("create execution evidence at self-test specs");
    fs::create_dir_all(&baseline).expect("create execution evidence at self-test baseline");
    write_text(
        &specs.join("spec.md"),
        "### Requirement: ADV-009\n#### Scenario: Positive - Present\n",
    )
    .expect("write execution evidence at self-test spec");
    let scenarios = read_scenarios(&specs).expect("read execution evidence at self-test scenarios");
    let manifest_path = root.join("manifest.tsv");
    write_text(
        &manifest_path,
        &render(&scenarios).expect("render execution evidence at self-test manifest"),
    )
    .expect("write execution evidence at self-test manifest");
    let manifest_path_string = manifest_path.to_string_lossy().into_owned();
    write_text(
        &baseline.join("spec.md"),
        "### Requirement: ADV-009\n#### Scenario: Positive - Present\n",
    )
    .expect("write execution evidence at self-test baseline spec");

    validate_execution_evidence_at(
        &root.join("missing-specs").to_string_lossy(),
        BASELINE_SPEC_ROOT,
        MANIFEST,
        V2_MANIFEST,
        V2_EXECUTIONS,
        false,
    )
    .expect_err("execution evidence at missing spec root control must fail");

    validate_execution_evidence_at(
        SPEC_ROOT,
        BASELINE_SPEC_ROOT,
        &root.join("missing-manifest.tsv").to_string_lossy(),
        V2_MANIFEST,
        V2_EXECUTIONS,
        false,
    )
    .expect_err("execution evidence at missing manifest control must fail");

    let stale_manifest_path = root.join("stale-manifest.tsv");
    write_text(&stale_manifest_path, "stale contract\n")
        .expect("write execution evidence at self-test stale manifest");
    validate_execution_evidence_at(
        specs.to_string_lossy().as_ref(),
        baseline.to_string_lossy().as_ref(),
        &stale_manifest_path.to_string_lossy(),
        V2_MANIFEST,
        V2_EXECUTIONS,
        false,
    )
    .expect_err("execution evidence at stale manifest control must fail");

    validate_execution_evidence_at(
        specs.to_string_lossy().as_ref(),
        &root.join("missing-baseline").to_string_lossy(),
        &manifest_path_string,
        V2_MANIFEST,
        V2_EXECUTIONS,
        false,
    )
    .expect_err("execution evidence at missing baseline root control must fail");

    validate_execution_evidence_at(
        specs.to_string_lossy().as_ref(),
        baseline.to_string_lossy().as_ref(),
        &manifest_path_string,
        V2_MANIFEST,
        &root.join("missing-v2-executions.tsv").to_string_lossy(),
        false,
    )
    .expect_err("execution evidence at missing v2 executions control must fail");

    validate_execution_evidence_at(
        specs.to_string_lossy().as_ref(),
        baseline.to_string_lossy().as_ref(),
        &manifest_path_string,
        &root.join("missing-v2-manifest.tsv").to_string_lossy(),
        V2_EXECUTIONS,
        false,
    )
    .expect_err("execution evidence at missing v2 manifest control must fail");

    let malformed_v2_manifest_path = root.join("malformed-v2-manifest.tsv");
    write_text(&malformed_v2_manifest_path, "header\ntoo\tfew\tfields\n")
        .expect("write execution evidence at self-test malformed v2 manifest");
    validate_execution_evidence_at(
        specs.to_string_lossy().as_ref(),
        baseline.to_string_lossy().as_ref(),
        &manifest_path_string,
        &malformed_v2_manifest_path.to_string_lossy(),
        V2_EXECUTIONS,
        false,
    )
    .expect_err("execution evidence at malformed v2 manifest content control must fail");

    let empty_v2_manifest_path = root.join("empty-v2-manifest.tsv");
    write_text(&empty_v2_manifest_path, "header\n")
        .expect("write execution evidence at self-test empty v2 manifest");
    validate_execution_evidence_at(
        specs.to_string_lossy().as_ref(),
        baseline.to_string_lossy().as_ref(),
        &manifest_path_string,
        &empty_v2_manifest_path.to_string_lossy(),
        V2_EXECUTIONS,
        false,
    )
    .expect_err("execution evidence at stale materialized evidence control must fail");

    fs::remove_dir_all(&root).expect("remove execution evidence at self-test directory");
}

fn validate_execution_evidence_records_controls() {
    let mut single = BTreeSet::new();
    single.insert(String::from("scenario-a"));
    let source = RequirementSource {
        path: String::from("Cargo.toml"),
        scenarios: single,
    };
    let mut sources = BTreeMap::new();
    sources.insert(String::from("ADV-009"), source);
    let empty_operations = BTreeMap::new();

    // Malformed hardening manifest row (wrong field count).
    validate_execution_evidence_records(
        &BTreeMap::new(),
        "header\nnot-nine-fields\n",
        "header\n",
        &empty_operations,
    )
    .expect_err("execution evidence malformed hardening row control must fail");

    // Malformed hardening manifest row (wrong schema, correct field count).
    validate_execution_evidence_records(
        &BTreeMap::new(),
        &format!(
            "header\n{}\n",
            manifest_row("ADV-009", "scenario-a").replacen(SCHEMA, "wrong-schema/v1", 1)
        ),
        "header\n",
        &empty_operations,
    )
    .expect_err("execution evidence wrong schema hardening row control must fail");

    // Unknown requirement in the hardening manifest.
    validate_execution_evidence_records(
        &BTreeMap::new(),
        &format!("header\n{}\n", manifest_row("ADV-009", "scenario-a")),
        "header\n",
        &empty_operations,
    )
    .expect_err("execution evidence unknown hardening requirement control must fail");

    // Undeclared scenario in the hardening manifest.
    validate_execution_evidence_records(
        &sources,
        &format!("header\n{}\n", manifest_row("ADV-009", "scenario-b")),
        "header\n",
        &empty_operations,
    )
    .expect_err("execution evidence undeclared hardening scenario control must fail");

    // Duplicate scenario in the hardening manifest.
    let duplicate_row = manifest_row("ADV-009", "scenario-a");
    validate_execution_evidence_records(
        &sources,
        &format!("header\n{duplicate_row}\n{duplicate_row}\n"),
        "header\n",
        &empty_operations,
    )
    .expect_err("execution evidence duplicate hardening scenario control must fail");

    // Hardening source path unreadable.
    let mut unreadable_source = BTreeMap::new();
    let mut unreadable_scenarios = BTreeSet::new();
    unreadable_scenarios.insert(String::from("scenario-a"));
    unreadable_source.insert(
        String::from("ADV-009"),
        RequirementSource {
            path: String::from("target/hardening-execution-evidence-controls-missing-file"),
            scenarios: unreadable_scenarios,
        },
    );
    validate_execution_evidence_records(
        &unreadable_source,
        &format!("header\n{}\n", manifest_row("ADV-009", "scenario-a")),
        "header\n",
        &empty_operations,
    )
    .expect_err("execution evidence unreadable hardening source control must fail");

    // Malformed V2 manifest row.
    validate_execution_evidence_records(
        &BTreeMap::new(),
        "header\n",
        "header\ntoo\tfew\tfields\n",
        &empty_operations,
    )
    .expect_err("execution evidence malformed v2 row control must fail");

    // Unknown requirement in the V2 manifest.
    validate_execution_evidence_records(
        &BTreeMap::new(),
        "header\n",
        &format!(
            "header\n{}\n",
            v2_manifest_row("ADV-009", "scenario-a", "fixA", "caseA")
        ),
        &empty_operations,
    )
    .expect_err("execution evidence unknown v2 requirement control must fail");

    // Missing separate operation result for a V2 manifest row.
    validate_execution_evidence_records(
        &sources,
        "header\n",
        &format!(
            "header\n{}\n",
            v2_manifest_row("ADV-009", "scenario-a", "fixA", "caseA")
        ),
        &empty_operations,
    )
    .expect_err("execution evidence missing v2 operation result control must fail");

    // Duplicate scenario in the V2 manifest.
    let mut operations = BTreeMap::new();
    operations.insert(
        (String::from("fixA"), String::from("caseA")),
        String::from("digest"),
    );
    let v2_row = v2_manifest_row("ADV-009", "scenario-a", "fixA", "caseA");
    validate_execution_evidence_records(
        &sources,
        "header\n",
        &format!("header\n{v2_row}\n{v2_row}\n"),
        &operations,
    )
    .expect_err("execution evidence duplicate v2 scenario control must fail");

    // V2 source path unreadable.
    validate_execution_evidence_records(
        &unreadable_source,
        "header\n",
        &format!(
            "header\n{}\n",
            v2_manifest_row("ADV-009", "scenario-a", "fixA", "caseA")
        ),
        &operations,
    )
    .expect_err("execution evidence unreadable v2 source control must fail");

    // No exact scenario authority for an unresolved requirement family.
    let mut unresolved_scenarios = BTreeSet::new();
    unresolved_scenarios.insert(String::from("scenario-a"));
    let mut unknown_family = BTreeMap::new();
    unknown_family.insert(
        String::from("FOO-001"),
        RequirementSource {
            path: String::from("Cargo.toml"),
            scenarios: unresolved_scenarios.clone(),
        },
    );
    validate_execution_evidence_records(&unknown_family, "header\n", "header\n", &empty_operations)
        .expect_err("execution evidence unknown authority family control must fail");

    // Fallback source path unreadable for a recognized authority family.
    let mut unreadable_fallback = BTreeMap::new();
    unreadable_fallback.insert(
        String::from("PID-001"),
        RequirementSource {
            path: String::from("target/hardening-execution-evidence-controls-missing-file"),
            scenarios: unresolved_scenarios,
        },
    );
    validate_execution_evidence_records(
        &unreadable_fallback,
        "header\n",
        "header\n",
        &empty_operations,
    )
    .expect_err("execution evidence unreadable fallback source control must fail");

    // Requirement id must be nonempty: an empty requirement key that is fully
    // resolved by the hardening manifest reaches the final per-requirement
    // execution-evidence record, whose id is the bare requirement string.
    let mut empty_requirement_scenarios = BTreeSet::new();
    empty_requirement_scenarios.insert(String::from("scenario-a"));
    let mut empty_requirement_sources = BTreeMap::new();
    empty_requirement_sources.insert(
        String::new(),
        RequirementSource {
            path: String::from("Cargo.toml"),
            scenarios: empty_requirement_scenarios,
        },
    );
    validate_execution_evidence_records(
        &empty_requirement_sources,
        &format!("header\n{}\n", manifest_row("", "scenario-a")),
        "header\n",
        &empty_operations,
    )
    .expect_err("execution evidence empty requirement id control must fail");

    // A requirement with no declared scenarios and no earlier reference never
    // gains a `requirement_results` entry.
    let mut no_scenario_sources = BTreeMap::new();
    no_scenario_sources.insert(
        String::from("ADV-009"),
        RequirementSource {
            path: String::from("Cargo.toml"),
            scenarios: BTreeSet::new(),
        },
    );
    validate_execution_evidence_records(
        &no_scenario_sources,
        "header\n",
        "header\n",
        &empty_operations,
    )
    .expect_err("execution evidence no scenario results control must fail");
}

fn command_controls() {
    assert_eq!(
        parse_command(&[]).expect("parse default command control"),
        Command::Manifest { write: false }
    );
    for (argument, expected) in [
        ("--write", Command::Manifest { write: true }),
        ("--self-test", Command::SelfTest),
        (
            "--check-execution-evidence",
            Command::ExecutionEvidence { write: false },
        ),
        (
            "--write-execution-evidence",
            Command::ExecutionEvidence { write: true },
        ),
        ("--coverage-evidence", Command::CoverageEvidence),
        (
            "--ledger",
            Command::Ledger {
                write: false,
                require_green: false,
            },
        ),
        (
            "--ledger-write",
            Command::Ledger {
                write: true,
                require_green: false,
            },
        ),
        (
            "--ledger-green",
            Command::Ledger {
                write: false,
                require_green: true,
            },
        ),
        ("--ledger-self-test", Command::LedgerSelfTest),
    ] {
        assert_eq!(
            parse_command(&[String::from(argument)]).expect("parse named command control"),
            expected
        );
    }
    run_command(Command::CoverageEvidence, || Ok(()))
        .expect("coverage evidence dispatch control must pass");
    assert_eq!(
        parse_command(&[String::from("--invalid")]).expect_err("usage control must fail"),
        USAGE
    );
}

fn repository_controls(scenarios: &[Scenario], valid: &str) {
    let root = env::temp_dir().join(format!(
        "buoy-hardening-manifest-self-test-{}",
        std::process::id()
    ));
    let _ = fs::remove_dir_all(&root);
    let specs = root.join("specs");
    fs::create_dir_all(&specs).expect("create hardening manifest self-test specs");
    write_text(
        &specs.join("spec.md"),
        "### Requirement: ADV-009\n\
         #### Scenario: Positive - Fixture mutation is executed and detected\n\
         #### Scenario: Negative - Manifest-only fixture passes without execution\n",
    )
    .expect("write hardening manifest self-test spec");
    write_text(&specs.join("notes.md"), "ignored non-spec control\n")
        .expect("write ignored hardening manifest self-test file");

    let parsed = read_scenarios(&specs).expect("read hardening manifest self-test scenarios");
    assert_eq!(parsed, scenarios);
    let manifest = root.join("hardening-manifest.tsv");
    validate_repository_at(&specs, &manifest, true)
        .expect("write valid hardening manifest control");
    validate_repository_at(&specs, &manifest, false)
        .expect("validate current hardening manifest control");

    let lines = valid.lines().collect::<Vec<_>>();
    let reordered = format!("{}\n{}\n{}\n", lines[0], lines[2], lines[1]);
    write_text(&manifest, &reordered).expect("write reordered hardening manifest control");
    validate_repository_at(&specs, &manifest, false).expect_err("stale manifest control must fail");

    write_text(&manifest, "").expect("write invalid hardening manifest control");
    validate_repository_scenarios(scenarios, &manifest, false)
        .expect_err("invalid manifest text control must fail");

    fs::remove_file(&manifest).expect("remove hardening manifest self-test manifest");
    validate_repository_scenarios(scenarios, &manifest, false)
        .expect_err("missing manifest control must fail");
    validate_repository_scenarios(scenarios, &manifest, true)
        .expect("restore valid hardening manifest control");

    validate_repository_scenarios(scenarios, &root.join("missing/manifest.tsv"), true)
        .expect_err("manifest write failure control must fail");
    validate_repository_at(&root.join("missing-specs"), &manifest, false)
        .expect_err("missing spec root control must fail");
    read_scenario_paths(vec![root.join("missing-file")])
        .expect_err("missing scenario file control must fail");
    directory_entry_path(&specs, Err(io::Error::other("directory entry control")))
        .expect_err("directory entry control must fail");
    collect_markdown_paths(
        vec![Err(String::from("directory entry control"))],
        &mut Vec::new(),
    )
    .expect_err("collected directory entry control must fail");
    collect_markdown_paths(
        vec![Ok((root.join("missing-directory"), true))],
        &mut Vec::new(),
    )
    .expect_err("recursive directory control must fail");
    validate_repository_scenarios(
        &[Scenario {
            requirement: String::from("UNKNOWN-001"),
            slug: String::from("unknown-authority"),
            polarity: String::from("negative"),
        }],
        &manifest,
        false,
    )
    .expect_err("repository authority mapping control must fail");
    validate_repository_scenarios(
        &[Scenario {
            requirement: String::from("UNKNOWN-001"),
            slug: String::from("unknown-authority"),
            polarity: String::from("negative"),
        }],
        &root.join("unknown-authority-write.tsv"),
        true,
    )
    .expect_err("repository authority mapping write control must fail");

    fs::remove_dir_all(&root).expect("remove hardening manifest self-test directory");
}

/// Every requirement this checker admits, derived rather than restated: the
/// requirement families are read out of the real committed manifest's
/// requirement column, and every candidate number in each family is then put
/// through the real `known_requirement` gate. REH-012 and REH-013 are
/// therefore included as soon as `known_requirement` admits them, before any
/// manifest row for them exists.
fn admitted_requirements() -> BTreeSet<String> {
    let manifest = read_text(Path::new(MANIFEST)).expect("read the committed hardening manifest");
    let families = manifest
        .lines()
        .skip(1)
        .filter(|line| !line.is_empty())
        .filter_map(|line| line.split('\t').nth(1))
        .filter_map(|requirement| requirement.split_once('-').map(|item| item.0.to_owned()))
        .collect::<BTreeSet<_>>();
    assert!(
        families.contains("REH"),
        "the committed hardening manifest must carry the REH family"
    );
    let mut admitted = BTreeSet::new();
    for family in &families {
        for number in 1..=99_u8 {
            let requirement = format!("{family}-{number:03}");
            if known_requirement(&requirement) {
                admitted.insert(requirement);
            }
        }
    }
    admitted
}

/// A declared focused command that omits a required feature flag is a vacuous
/// oracle: `cargo test --test publication_filesystem <name>` compiles and runs
/// ZERO tests when the named oracle sits behind `coverage-controls` or
/// `blake3-256`, yet still exits successfully. This control closes that hole
/// for every requirement the checker admits -- including REH-012 and REH-013,
/// whose rows do not exist in the standing manifest yet -- and at the same
/// time proves the authority mapping is closed all the way through REH-013.
fn focused_command_feature_controls() {
    let admitted = admitted_requirements();
    for requirement in ["REH-011", "REH-012", "REH-013"] {
        assert!(
            admitted.contains(requirement),
            "{requirement} must be an admitted hardening requirement"
        );
    }
    assert!(
        !admitted.contains("REH-014"),
        "REH-014 is not declared and must not be admitted"
    );

    let mut cargo_commands = 0_usize;
    for requirement in &admitted {
        let oracle = independent_oracle(requirement);
        let command = focused_command(requirement);
        validate_authority_mapping(requirement, oracle, command)
            .expect("every admitted hardening requirement needs a closed authority mapping");
        if command.contains("cargo test") {
            cargo_commands += 1;
            assert!(
                command.contains("--all-features"),
                "{requirement} declares focused command {command:?}, which compiles only the \
                 default feature set and can therefore execute zero tests"
            );
            assert!(
                command.contains("--test "),
                "{requirement} declares focused command {command:?}, which names no test target"
            );
        }
    }
    assert!(
        cargo_commands >= 3,
        "the feature control must actually inspect real cargo test commands, saw {cargo_commands}"
    );

    // Discrimination control: the same rule really does reject a command that
    // drops the feature flag, so the loop above is not passing vacuously.
    let complete = focused_command("REH-012");
    assert!(complete.contains("--all-features"));
    assert!(
        !complete
            .replacen(" --all-features", "", 1)
            .contains("--all-features")
    );
}

fn scenario_controls() {
    let root = env::temp_dir().join(format!(
        "buoy-hardening-scenario-self-test-{}",
        std::process::id()
    ));
    for text in [
        "### Requirement:    \n",
        "### Requirement: UNKNOWN-001\n",
        "#### Scenario: Positive - Before requirement\n",
        "### Requirement: ADV-009\n#### Scenario: Unclosed polarity\n",
        "### Requirement: ADV-009\n#### Scenario: Positive - !!!\n",
        "### Requirement: ADV-009\n",
        "### Requirement: ADV-009\n\
         #### Scenario: Positive - Duplicate\n\
         #### Scenario: Positive - Duplicate\n",
    ] {
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(&root).expect("create hardening scenario self-test directory");
        write_text(&root.join("spec.md"), text).expect("write hardening scenario control");
        read_scenarios(&root).expect_err("hardening scenario negative control must fail");
    }
    fs::remove_dir_all(&root).expect("remove hardening scenario self-test directory");
}

fn requirement_source_controls() {
    let mut sources = BTreeMap::new();
    requirement_source_lines(
        "### Requirement: ADV-009\n#### Scenario: Positive - Present\n",
        "control.md",
        &mut sources,
    )
    .expect("requirement source lines valid control");
    assert!(sources.contains_key("ADV-009"));

    for text in [
        "### Requirement:    \n",
        "### Requirement: ADV-009\n### Requirement: ADV-009\n",
        "#### Scenario: Positive - Before requirement\n",
        "### Requirement: ADV-009\n#### Scenario: Unclosed polarity\n",
        "### Requirement: ADV-009\n\
         #### Scenario: Positive - Duplicate\n\
         #### Scenario: Positive - Duplicate\n",
    ] {
        let mut sources = BTreeMap::new();
        requirement_source_lines(text, "control.md", &mut sources)
            .expect_err("requirement source lines negative control must fail");
    }

    requirement_sources_at(&["target/hardening-requirement-sources-missing-root"])
        .expect_err("requirement sources missing root control must fail");

    let root = env::temp_dir().join(format!(
        "buoy-hardening-requirement-source-self-test-{}",
        std::process::id()
    ));
    let _ = fs::remove_dir_all(&root);
    requirement_sources_from_paths(vec![root.join("missing-spec.md")])
        .expect_err("requirement sources unreadable file control must fail");

    fs::create_dir_all(&root).expect("create requirement source paths self-test directory");
    let malformed = root.join("malformed-spec.md");
    write_text(&malformed, "#### Scenario: Positive - Before requirement\n")
        .expect("write requirement source paths self-test spec");
    requirement_sources_from_paths(vec![malformed])
        .expect_err("requirement sources malformed file control must fail");
    fs::remove_dir_all(&root).expect("remove requirement source paths self-test directory");
}
