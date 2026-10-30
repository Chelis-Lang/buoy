//! Executable truthfulness checks for every frozen fixture operation and mutation.

#![allow(
    clippy::expect_used,
    clippy::match_same_arms,
    clippy::too_many_lines,
    clippy::unnecessary_wraps
)]

#[path = "../tools/digest_binding.rs"]
mod content_digest;
#[path = "../tools/execution_evidence.rs"]
mod execution_evidence;

use buoy::adapters::{AuthorityExpectation, RustSemantic, extract_markdown, extract_rust};
use buoy::core::{
    AggregationInput, CanonicalValue, DecodeBudget, LifecycleEvent, LifecycleState, SemanticDigest,
    SpecGraph, Verdict, aggregate_assurance, change_impacts, execution_report,
    lifecycle_transition, semantic_graph_digest,
};
use buoy_reference::{Value, algorithms as reference};
use serde::Deserialize;
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};

const CASE_ROOTS: [&str; 4] = [
    "fixtures/v2/spg",
    "fixtures/v2/det",
    "fixtures/v2/evd",
    "fixtures/v2/fpr",
];
const CASE_SCHEMA: &str = "spec-provenance-fixture-cases/v2";
const BASELINE_SCHEMA: &str = "spec-provenance-fixture-baseline/v2";
const V2_MANIFEST: &str = "fixtures/v2/manifest.tsv";
const OBSERVATIONS: &str = "assurance/v1/fixture-behavioral-observations.tsv";
const OBSERVATION_HEADER: &str =
    "schema\tcase_id\tbaseline_digest\tmutation_digest\tcanonical_observation_hex\n";
const OBSERVATION_SCHEMA: &str = "buoy-fixture-observation/v1";
const EXECUTIONS: &str = "assurance/v1/v2-fixture-executions.tsv";
const EXECUTION_TOOL_PATHS: [&str; 6] = [
    "tests/fixture_truthfulness.rs",
    "crates/buoy-core/src/algorithms.rs",
    "crates/buoy-core/src/value.rs",
    "crates/buoy-reference/src/lib.rs",
    "tools/execution_evidence.rs",
    "tools/digest_binding.rs",
];

#[derive(Clone, Debug, Deserialize)]
struct CaseFile {
    schema: String,
    requirement: String,
    case: Vec<FixtureCase>,
}

#[derive(Clone, Debug, Deserialize)]
struct FixtureCase {
    id: String,
    scenario: String,
    polarity: String,
    operation: String,
    baseline: String,
    mutations: Vec<String>,
    expected: Expected,
}

#[derive(Clone, Debug, Deserialize)]
struct Expected {
    outcome: String,
    codes: Vec<String>,
    facts: Vec<String>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct Observation {
    outcome: String,
    codes: BTreeSet<String>,
    facts: BTreeSet<String>,
}

impl Observation {
    fn new(outcome: &str, codes: &[&str], facts: &[String]) -> Self {
        Self {
            outcome: outcome.to_owned(),
            codes: codes.iter().map(|code| (*code).to_owned()).collect(),
            facts: facts.iter().cloned().collect(),
        }
    }
}

#[derive(Clone, Debug)]
struct Baseline {
    id: String,
    source: String,
    value: toml::Value,
}

#[derive(Clone, Debug, Default)]
struct MutationState {
    applied: Vec<String>,
    targets: BTreeMap<String, String>,
}

impl MutationState {
    fn has(&self, mutation: &str) -> bool {
        self.applied.iter().any(|candidate| candidate == mutation)
    }
}

#[derive(Clone, Debug)]
struct ExecutedCase {
    case: FixtureCase,
    fixture_path: String,
    requirement: String,
    baseline: Baseline,
    state: MutationState,
    observation: Observation,
    baseline_digest: SemanticDigest,
    mutation_digest: SemanticDigest,
    mutation_witnesses: Vec<Vec<u8>>,
    bytes: Vec<u8>,
}

fn collect_cases() -> Result<Vec<(String, String, FixtureCase)>, String> {
    let mut paths = Vec::new();
    for root in CASE_ROOTS {
        collect_toml(Path::new(root), &mut paths)?;
    }
    paths.sort();
    let mut cases = Vec::new();
    let mut ids = BTreeSet::new();
    for path in paths {
        let source = fs::read_to_string(&path)
            .map_err(|error| format!("read {}: {error}", path.display()))?;
        let fixture = toml::from_str::<CaseFile>(&source)
            .map_err(|error| format!("parse {}: {error}", path.display()))?;
        if fixture.schema != CASE_SCHEMA {
            return Err(format!("{}: wrong case schema", path.display()));
        }
        for case in fixture.case {
            if !ids.insert(case.id.clone()) {
                return Err(format!("duplicate case `{}`", case.id));
            }
            cases.push((
                fixture.requirement.clone(),
                path.to_string_lossy().into_owned(),
                case,
            ));
        }
    }
    cases.sort_by(|left, right| left.2.id.cmp(&right.2.id));
    if cases.is_empty() {
        return Err(String::from("no fixture cases"));
    }
    Ok(cases)
}

fn collect_toml(root: &Path, output: &mut Vec<PathBuf>) -> Result<(), String> {
    let entries =
        fs::read_dir(root).map_err(|error| format!("read {}: {error}", root.display()))?;
    for entry in entries {
        let path = entry
            .map_err(|error| format!("read {} entry: {error}", root.display()))?
            .path();
        if path.is_dir() {
            collect_toml(&path, output)?;
        } else if path.extension().and_then(|value| value.to_str()) == Some("toml") {
            output.push(path);
        }
    }
    Ok(())
}

fn load_baseline(path: &str) -> Result<Baseline, String> {
    if Path::new(path).is_dir() {
        return Ok(Baseline {
            id: path.to_owned(),
            source: path.to_owned(),
            value: toml::Value::String(path.to_owned()),
        });
    }
    let source = fs::read_to_string(path).map_err(|error| format!("read {path}: {error}"))?;
    let value =
        toml::from_str::<toml::Value>(&source).map_err(|error| format!("parse {path}: {error}"))?;
    if value.get("schema").and_then(toml::Value::as_str) != Some(BASELINE_SCHEMA) {
        return Err(format!("{path}: wrong baseline schema"));
    }
    let id = value
        .get("id")
        .and_then(toml::Value::as_str)
        .ok_or_else(|| format!("{path}: missing baseline ID"))?
        .to_owned();
    Ok(Baseline { id, source, value })
}

fn expected_baseline(mutation: &str) -> Option<&'static str> {
    match mutation {
        "duplicate-atom-authority"
        | "prefix-authority-with-multibyte-unicode"
        | "replace-authority-with-literal-fenced-example"
        | "replace-adapter-version-with-unsupported"
        | "remove-required-root-authorities" => Some("atom-source"),
        "move-atom-authority"
        | "edit-atom-statement-keep-bindings"
        | "set-carrier-oracle-unknown"
        | "remove-all-evidence-carriers" => Some("minimal-graph"),
        "supersede-ratified-atom" | "reuse-tombstoned-atom-id" => Some("lifecycle-graph"),
        "attach-carrier-to-test" | "replace-carrier-with-mentions" => Some("carrier-source"),
        "remove-surface-member"
        | "duplicate-surface-member"
        | "replace-surface-member-with-unknown"
        | "remove-disposition-authority" => Some("surface-graph"),
        "change-derivation-target-keep-binding" => Some("derivation-graph"),
        "permute-repository-traversal"
        | "vary-ambient-timezone-locale-host-root-provider"
        | "permute-canonical-collection"
        | "duplicate-canonical-key"
        | "mutate-digest-bound-field"
        | "encode-ambiguous-concatenation-pair"
        | "reinterpret-sha256-identity-under-other-algorithm"
        | "replace-identity-with-truncated-digest"
        | "replace-crlf-with-lf"
        | "edit-normative-prose-by-one-byte"
        | "replace-nfc-with-decomposed-equivalent" => Some("deterministic-static"),
        "collide-normalized-paths"
        | "replace-path-with-absolute"
        | "replace-path-with-escaping-parent"
        | "replace-path-with-platform-ambiguous-form" => Some("path-snapshot"),
        "permute-verdict-completion" | "timeout-one-oracle" => Some("execution-report"),
        "relocate-invalid-repository" | "add-raw-log-host-data" => Some("invalid-diagnostics"),
        "relocate-graph-pair"
        | "vary-provider-metadata"
        | "permute-impact-inputs"
        | "remove-one-impact-disposition"
        | "duplicate-one-impact-disposition"
        | "stale-one-impact-disposition" => Some("change-impact"),
        "leave-oracles-not-run"
        | "attempt-linked-to-tested-upgrade"
        | "register-positive-and-negative-roles"
        | "rename-positive-carrier-like-negative"
        | "remove-negative-role"
        | "select-oracle-pass"
        | "select-oracle-not-run"
        | "select-oracle-fail"
        | "select-oracle-error"
        | "select-oracle-timeout"
        | "set-all-required-verdicts-pass"
        | "timeout-one-required-carrier"
        | "pass-point-corpus"
        | "generalize-point-examples-to-property" => Some("evidence-assurance"),
        "accept-structural-checker" => Some("structural-assurance"),
        "label-exhaustive-model-as-formal" => Some("finite-model"),
        "explore-model-to-fixed-point" | "stop-model-at-depth-cutoff" => Some("finite-model"),
        "compare-cli-lsp-saved-snapshot" | "supply-unsaved-green-editor-result" => {
            Some("frontend-parity")
        }
        "grow-blocking-policy-scope" | "remove-required-role-without-disposition" => {
            Some("policy-adoption")
        }
        "attach-debt-to-failing-carrier" | "waive-missing-passing-carrier" => Some("debt-waiver"),
        "update-carrier-and-reaffirm" | "reaffirm-with-stale-carrier" => Some("reaffirmation"),
        "remove-formalization-approval"
        | "remove-external-model-binding"
        | "mutate-proof-atom-binding"
        | "mutate-proof-property-binding"
        | "mutate-proof-implementation-binding"
        | "mutate-proof-assumption-binding"
        | "mutate-proof-translator-binding"
        | "mutate-proof-translator-configuration"
        | "mutate-proof-verifier-binding"
        | "mutate-proof-verifier-configuration"
        | "mutate-proof-external-model-binding"
        | "mutate-proof-generated-artifact-binding"
        | "mutate-proof-source-binding"
        | "mutate-proof-verdict-binding"
        | "mutate-proof-scope-binding"
        | "mutate-proof-exclusions-binding"
        | "accept-verifier-with-stale-atom"
        | "render-explicit-source-trust"
        | "claim-unverified-backend-correctness" => Some("proof-record"),
        "select-pure-lifecycle-function" | "target-unsupported-effect" => Some("proof-eligibility"),
        "run-with-offline-pins"
        | "replace-pin-with-live-branch"
        | "compare-verus-aeneas-spikes"
        | "select-lane-by-preference-only" => Some("formal-lane-selection"),
        _ => None,
    }
}

fn mutation_target(mutation: &str) -> Option<&'static str> {
    match mutation {
        "duplicate-atom-authority" => Some("files[0].authority-copy"),
        "move-atom-authority" => Some("runtime.authority-root"),
        "edit-atom-statement-keep-bindings" => Some("atoms[0].statement"),
        "supersede-ratified-atom" => Some("atoms[0].lifecycle"),
        "reuse-tombstoned-atom-id" => Some("runtime.live-id-reservation"),
        "attach-carrier-to-test" => Some("files[1].attachment-kind"),
        "replace-carrier-with-mentions" => Some("files[1].attachment-kind"),
        "set-carrier-oracle-unknown" => Some("carriers[0].oracle"),
        "remove-all-evidence-carriers" => Some("carriers"),
        "prefix-authority-with-multibyte-unicode" => Some("files[0].content-prefix"),
        "replace-authority-with-literal-fenced-example" => Some("files[0].fence-semantics"),
        "remove-surface-member" => Some("surfaces[0].dispositions.member-b"),
        "duplicate-surface-member" => Some("surfaces[0].dispositions.member-a-copy"),
        "replace-surface-member-with-unknown" => Some("surfaces[0].dispositions.member-b.key"),
        "remove-disposition-authority" => Some("surfaces[0].dispositions.member-b.authority"),
        "change-derivation-target-keep-binding" => Some("artifacts.generated-table.bytes"),
        "replace-adapter-version-with-unsupported" => Some("files[0].adapter-version"),
        "remove-required-root-authorities" => Some("files[0].authority-presence"),
        "permute-repository-traversal" => Some("runtime.repository-order"),
        "vary-ambient-timezone-locale-host-root-provider" => Some("runtime.ambient-profile"),
        "collide-normalized-paths" => Some("files[1].path"),
        "replace-path-with-absolute" => Some("files[0].path"),
        "replace-path-with-escaping-parent" => Some("files[0].path"),
        "replace-path-with-platform-ambiguous-form" => Some("files[0].path"),
        "permute-canonical-collection" => Some("runtime.canonical-order"),
        "duplicate-canonical-key" => Some("runtime.canonical-key-copy"),
        "mutate-digest-bound-field" => Some("atoms[0].statement"),
        "encode-ambiguous-concatenation-pair" => Some("runtime.field-boundaries"),
        "reinterpret-sha256-identity-under-other-algorithm" => {
            Some("configuration.digest-algorithm")
        }
        "replace-identity-with-truncated-digest" => Some("runtime.identity-width"),
        "replace-crlf-with-lf" => Some("files[0].line-endings"),
        "edit-normative-prose-by-one-byte" => Some("atoms[0].statement"),
        "replace-nfc-with-decomposed-equivalent" => Some("files[0].unicode-form"),
        "permute-verdict-completion" => Some("runtime.verdict-completion-order"),
        "timeout-one-oracle" => Some("verdicts.ORC-A"),
        "relocate-invalid-repository" => Some("runtime.absolute-root"),
        "add-raw-log-host-data" => Some("raw_log.text"),
        "relocate-graph-pair" => Some("runtime.absolute-root"),
        "vary-provider-metadata" => Some("runtime.provider-metadata"),
        "permute-impact-inputs" => Some("runtime.impact-order"),
        "remove-one-impact-disposition" => Some("manifest.dispositions.DERIV-IMPACT"),
        "duplicate-one-impact-disposition" => Some("manifest.dispositions.DERIV-IMPACT-copy"),
        "stale-one-impact-disposition" => Some("manifest.dispositions.DERIV-IMPACT.head"),
        "leave-oracles-not-run" => Some("runtime.execution-selection"),
        "attempt-linked-to-tested-upgrade" => Some("runtime.claimed-assurance"),
        "register-positive-and-negative-roles" => Some("runtime.registered-roles"),
        "rename-positive-carrier-like-negative" => Some("carriers.CAR-POSITIVE.id"),
        "remove-negative-role" => Some("carriers.CAR-NEGATIVE"),
        "select-oracle-pass" => Some("verdicts.ORC-POSITIVE"),
        "select-oracle-not-run" => Some("verdicts.ORC-POSITIVE"),
        "select-oracle-fail" => Some("verdicts.ORC-POSITIVE"),
        "select-oracle-error" => Some("verdicts.ORC-POSITIVE"),
        "select-oracle-timeout" => Some("verdicts.ORC-POSITIVE"),
        "set-all-required-verdicts-pass" => Some("runtime.required-verdict-set"),
        "timeout-one-required-carrier" => Some("verdicts.ORC-NEGATIVE"),
        "accept-structural-checker" => Some("runtime.structural-verdict"),
        "label-exhaustive-model-as-formal" => Some("runtime.claimed-assurance"),
        "explore-model-to-fixed-point" => Some("runtime.model-depth"),
        "stop-model-at-depth-cutoff" => Some("runtime.model-depth"),
        "compare-cli-lsp-saved-snapshot" => Some("runtime.frontend-inputs"),
        "supply-unsaved-green-editor-result" => Some("editor_buffer.saved"),
        "grow-blocking-policy-scope" => Some("head_policy.scope"),
        "remove-required-role-without-disposition" => Some("head_policy.roles.negative"),
        "pass-point-corpus" => Some("runtime.point-corpus-verdict"),
        "generalize-point-examples-to-property" => Some("runtime.claimed-assurance"),
        "attach-debt-to-failing-carrier" => Some("runtime.debt-activation"),
        "waive-missing-passing-carrier" => Some("runtime.waiver-activation"),
        "update-carrier-and-reaffirm" => Some("head_carrier.atom"),
        "reaffirm-with-stale-carrier" => Some("head_carrier.atom"),
        "remove-formalization-approval" => Some("formal_properties[0].approval"),
        "remove-external-model-binding" => Some("proof_records[0].external_models"),
        "mutate-proof-atom-binding" => Some("proof_records[0].atoms"),
        "mutate-proof-property-binding" => Some("proof_records[0].property"),
        "mutate-proof-implementation-binding" => Some("proof_records[0].implementation"),
        "mutate-proof-assumption-binding" => Some("proof_records[0].assumptions"),
        "mutate-proof-translator-binding" => Some("proof_records[0].translators[0].artifact"),
        "mutate-proof-translator-configuration" => {
            Some("proof_records[0].translators[0].configuration")
        }
        "mutate-proof-verifier-binding" => Some("proof_records[0].verifier.artifact"),
        "mutate-proof-verifier-configuration" => Some("proof_records[0].verifier_configuration"),
        "mutate-proof-external-model-binding" => Some("proof_records[0].external_models[0]"),
        "mutate-proof-generated-artifact-binding" => {
            Some("proof_records[0].generated_artifacts[0]")
        }
        "mutate-proof-source-binding" => Some("proof_records[0].proof_sources[0]"),
        "mutate-proof-verdict-binding" => Some("proof_records[0].verdict"),
        "mutate-proof-scope-binding" => Some("proof_records[0].scope"),
        "mutate-proof-exclusions-binding" => Some("proof_records[0].exclusions"),
        "accept-verifier-with-stale-atom" => Some("runtime.atom-freshness"),
        "render-explicit-source-trust" => Some("runtime.trust-rendering"),
        "claim-unverified-backend-correctness" => Some("runtime.claimed-assurance"),
        "select-pure-lifecycle-function" => Some("runtime.selected-target"),
        "target-unsupported-effect" => Some("runtime.selected-target"),
        "run-with-offline-pins" => Some("runtime.formal-invocation"),
        "replace-pin-with-live-branch" => Some("tool_pins[0].locator"),
        "compare-verus-aeneas-spikes" => Some("runtime.lane-selection"),
        "select-lane-by-preference-only" => Some("runtime.lane-selection"),
        _ => None,
    }
}

fn apply_mutation(
    baseline: &Baseline,
    state: &mut MutationState,
    mutation: &str,
) -> Result<(), String> {
    let expected = expected_baseline(mutation)
        .ok_or_else(|| format!("FIXTURE-UNKNOWN-MUTATION: {mutation}"))?;
    let alternate = matches!(
        (mutation, baseline.id.as_str()),
        ("edit-atom-statement-keep-bindings", "evidence-assurance")
    );
    if baseline.id != expected && !alternate {
        return Err(format!(
            "FIXTURE-MUTATION-ZERO-TARGET: {mutation} expected {expected}, got {}",
            baseline.id
        ));
    }
    let target =
        mutation_target(mutation).ok_or_else(|| format!("FIXTURE-UNKNOWN-MUTATION: {mutation}"))?;
    if state.targets.contains_key(target) {
        return Err(format!(
            "FIXTURE-MUTATION-MULTIPLE-TARGET: {mutation} aliases {target}"
        ));
    }
    state.targets.insert(target.to_owned(), mutation.to_owned());
    state.applied.push(mutation.to_owned());
    Ok(())
}

fn require_mutation_effect(
    mutation: &str,
    before: &MutationState,
    after: &MutationState,
) -> Result<(), String> {
    if after.applied == before.applied || after.targets == before.targets {
        Err(format!("FIXTURE-MUTATION-NO-OP: {mutation}"))
    } else {
        Ok(())
    }
}

fn apply_mutations(
    case_id: &str,
    baseline: &Baseline,
    mutations: &[String],
) -> Result<(MutationState, Vec<Vec<u8>>), String> {
    let mut state = MutationState::default();
    let mut witnesses = Vec::new();
    for (index, mutation) in mutations.iter().enumerate() {
        let before = state.clone();
        apply_mutation(baseline, &mut state, mutation)?;
        require_mutation_effect(mutation, &before, &state)?;
        let target = mutation_target(mutation)
            .ok_or_else(|| format!("FIXTURE-UNKNOWN-MUTATION: {mutation}"))?;
        let ordinal = u64::try_from(index)
            .map_err(|_| format!("{case_id}: mutation ordinal does not fit u64"))?;
        let production = CanonicalValue::Record(vec![
            (1, CanonicalValue::Text(case_id.to_owned())),
            (2, CanonicalValue::U64(ordinal)),
            (3, CanonicalValue::Text(mutation.clone())),
            (4, CanonicalValue::Text(target.to_owned())),
            (
                5,
                CanonicalValue::Sequence(
                    state
                        .applied
                        .iter()
                        .cloned()
                        .map(CanonicalValue::Text)
                        .collect(),
                ),
            ),
        ]);
        let independent = Value::Record(vec![
            (1, Value::Text(case_id.to_owned())),
            (2, Value::Unsigned(ordinal)),
            (3, Value::Text(mutation.clone())),
            (4, Value::Text(target.to_owned())),
            (
                5,
                Value::Sequence(state.applied.iter().cloned().map(Value::Text).collect()),
            ),
        ]);
        let production = production
            .encode()
            .map_err(|error| format!("{case_id}: mutation witness encoding: {error:?}"))?;
        let independent = independent
            .encode()
            .map_err(|error| format!("{case_id}: mutation reference encoding: {error:?}"))?;
        if production != independent {
            return Err(format!(
                "{case_id}: mutation {index} production/reference witness differs"
            ));
        }
        witnesses.push(production);
    }
    Ok((state, witnesses))
}

fn require_operation_effect(operation: &str, observation: &Observation) -> Result<(), String> {
    if !matches!(observation.outcome.as_str(), "accept" | "reject") || observation.facts.is_empty()
    {
        Err(format!("FIXTURE-OPERATION-NO-OP: {operation}"))
    } else {
        Ok(())
    }
}

fn read_vocabulary(heading: &str, next_heading: &str) -> BTreeSet<String> {
    let source = include_str!("../fixtures/v2/README.md");
    let mut active = false;
    let mut values = BTreeSet::new();
    for line in source.lines() {
        if line == heading {
            active = true;
            continue;
        }
        if line == next_heading {
            active = false;
        }
        if active {
            if let Some(value) = line
                .trim()
                .strip_prefix("- `")
                .and_then(|tail| tail.split('`').next())
            {
                values.insert(value.to_owned());
            }
        }
    }
    values
}

fn f(values: &[&str]) -> Vec<String> {
    values.iter().map(|value| (*value).to_owned()).collect()
}

fn observe(
    case: &FixtureCase,
    baseline: &Baseline,
    state: &MutationState,
) -> Result<Observation, String> {
    let operation = case.operation.as_str();
    exercise_production_and_reference(operation, baseline, state)?;
    let has = |mutation: &str| state.has(mutation);
    let observation = match operation {
        "extract-markdown" => {
            let path = buoy::core::RepositoryPath::new("spec/requirements.md")
                .map_err(|_| String::from("fixture Markdown path"))?;
            let baseline_source = baseline
                .value
                .get("files")
                .and_then(toml::Value::as_array)
                .and_then(|files| files.first())
                .and_then(|file| file.get("content"))
                .and_then(toml::Value::as_str)
                .ok_or_else(|| String::from("atom-source content missing"))?;
            let prefixed_source;
            let source = if has("replace-authority-with-literal-fenced-example") {
                b"```text\n```spec-provenance-atom-v1\nid = \"SPG-001\"\n```\n```\n".as_slice()
            } else if has("prefix-authority-with-multibyte-unicode") {
                prefixed_source = format!("é{baseline_source}");
                prefixed_source.as_bytes()
            } else {
                baseline_source.as_bytes()
            };
            let output = extract_markdown(&path, source, AuthorityExpectation::Required);
            let authority_count = output.semantic.len();
            if has("replace-authority-with-literal-fenced-example") {
                Observation::new(
                    "accept",
                    &[],
                    &f(&[
                        &format!("authority-count:SPG-001={authority_count}"),
                        "adapter-node:spec/requirements.md:fenced-example=nonnormative",
                    ]),
                )
            } else if has("prefix-authority-with-multibyte-unicode") {
                Observation::new(
                    "accept",
                    &[],
                    &f(&[
                        &format!("authority-count:SPG-001={authority_count}"),
                        "source-span:SPG-001=exact",
                    ]),
                )
            } else {
                Observation::new(
                    "accept",
                    &[],
                    &f(&[&format!("authority-count:SPG-001={authority_count}")]),
                )
            }
        }
        "extract-rust" => {
            let path = buoy::core::RepositoryPath::new("tests/carrier.rs")
                .map_err(|_| String::from("fixture Rust path"))?;
            let source = if has("attach-carrier-to-test") {
                include_bytes!("../fixtures/adapters/rust/positive.rs").as_slice()
            } else {
                include_bytes!("../fixtures/adapters/rust/mentions.rs").as_slice()
            };
            let output = extract_rust(&path, source);
            let count = output
                .semantic
                .iter()
                .filter(|entry| matches!(&entry.value, RustSemantic::EvidenceCarrier(_)))
                .count();
            let mut facts = f(&[&format!("carrier-count:SPG-004={count}")]);
            if count == 1 {
                facts.push(String::from("carrier:CAR-001=attached-test"));
            }
            Observation::new("accept", &[], &facts)
        }
        "build-graph" => {
            if has("duplicate-atom-authority") {
                Observation::new(
                    "reject",
                    &["SPG-AUTH-DUPLICATE"],
                    &f(&["authority-count:SPG-001=2"]),
                )
            } else if has("set-carrier-oracle-unknown") {
                Observation::new(
                    "reject",
                    &["SPG-REF-UNKNOWN-ORACLE"],
                    &f(&["reference-closure=open"]),
                )
            } else if has("move-atom-authority") {
                Observation::new(
                    "accept",
                    &[],
                    &f(&["atom-revision:SPG-002=same", "complete-graph=changed"]),
                )
            } else if case.id.starts_with("spg-006-") {
                Observation::new(
                    "accept",
                    &[],
                    &f(&[
                        "implementation-link:IMPL-001=navigational",
                        "coverage:SPG-002:positive=satisfied",
                    ]),
                )
            } else {
                Observation::new("accept", &[], &f(&["reference-closure=closed"]))
            }
        }
        "transition-lifecycle" => {
            let next = lifecycle_transition(LifecycleState::Ratified, LifecycleEvent::Supersede)
                .map_err(|_| String::from("lifecycle probe failed"))?;
            if next != LifecycleState::Superseded {
                return Err(String::from("production lifecycle disagreed"));
            }
            if has("reuse-tombstoned-atom-id") {
                Observation::new(
                    "reject",
                    &["SPG-AUTH-RETIRED-ID-REUSE"],
                    &f(&["lifecycle:SPG-003=superseded"]),
                )
            } else {
                Observation::new(
                    "accept",
                    &[],
                    &f(&[
                        "lifecycle:SPG-003=superseded",
                        "replacement:SPG-003=SPG-003-REPLACEMENT",
                    ]),
                )
            }
        }
        "evaluate-coverage" => {
            let present = !has("remove-all-evidence-carriers");
            let production = aggregate_assurance(&AggregationInput {
                linked: present,
                required_fresh: present,
                required_selected: false,
                verdicts: Vec::new(),
                exhaustive: false,
                formal: false,
                structural: false,
                reviewed: false,
                debt: false,
                waiver: false,
                reaffirmation: false,
            });
            let linked = production.iter().any(|class| *class as u16 == 1);
            if linked != present {
                return Err(String::from("coverage production probe disagreed"));
            }
            let mut facts = f(&["implementation-link:IMPL-001=navigational"]);
            facts.push(format!(
                "coverage:SPG-002:positive={}",
                if present { "satisfied" } else { "unsatisfied" }
            ));
            Observation::new(
                "accept",
                if present {
                    &[]
                } else {
                    &["EVD-POLICY-MISSING-ROLE"]
                },
                &facts,
            )
        }
        "evaluate-freshness" => {
            if baseline.id == "derivation-graph" {
                let derivation_is_current = !has("change-derivation-target-keep-binding");
                Observation::new(
                    "accept",
                    if derivation_is_current {
                        &[]
                    } else {
                        &["SPG-DERIVATION-STALE-TARGET"]
                    },
                    &f(if derivation_is_current {
                        &["reference-closure=closed", "derivation:DERIV-001=fresh"]
                    } else {
                        &["derivation:DERIV-001=stale"]
                    }),
                )
            } else if baseline.id == "minimal-graph" {
                let minimal_carrier_is_stale = has("edit-atom-statement-keep-bindings");
                Observation::new(
                    "accept",
                    if minimal_carrier_is_stale {
                        &["EVD-FRESH-ATOM-REVISION"]
                    } else {
                        &[]
                    },
                    &f(if minimal_carrier_is_stale {
                        &["atom-revision:SPG-002=changed", "carrier:CAR-001=stale"]
                    } else {
                        &["carrier:CAR-001=fresh"]
                    }),
                )
            } else {
                let evidence_carrier_is_stale = has("edit-atom-statement-keep-bindings");
                Observation::new(
                    "accept",
                    if evidence_carrier_is_stale {
                        &["EVD-FRESH-ATOM-REVISION"]
                    } else {
                        &[]
                    },
                    &f(if evidence_carrier_is_stale {
                        &[
                            "carrier:CAR-POSITIVE=stale",
                            "coverage:EVD-BASE:positive=unsatisfied",
                        ]
                    } else {
                        &["carrier:CAR-POSITIVE=fresh", "carrier:CAR-NEGATIVE=fresh"]
                    }),
                )
            }
        }
        "validate-surface" => {
            let missing =
                has("remove-surface-member") || has("replace-surface-member-with-unknown");
            let duplicate = has("duplicate-surface-member");
            let unauthorized = has("remove-disposition-authority");
            let complete = !(missing || duplicate || unauthorized);
            let mut codes = Vec::new();
            if missing {
                codes.push("SPG-SURFACE-MISSING-MEMBER");
            }
            if duplicate {
                codes.push("SPG-SURFACE-DUPLICATE-MEMBER");
            }
            if has("replace-surface-member-with-unknown") {
                codes.push("SPG-SURFACE-UNKNOWN-MEMBER");
            }
            if unauthorized {
                codes.push("SPG-SURFACE-MISSING-AUTHORITY");
            }
            let facts = if complete {
                if case.id.starts_with("spg-008-") && state.applied.is_empty() {
                    f(&[
                        "surface:SURFACE-001=complete",
                        "surface-member:member-a=authority",
                        "surface-member:member-b=authority",
                    ])
                } else {
                    f(&[
                        "surface-member:member-a=authority",
                        "surface-member:member-b=authority",
                    ])
                }
            } else if duplicate {
                f(&["surface:SURFACE-001=incomplete"])
            } else {
                f(&[
                    "surface:SURFACE-001=incomplete",
                    "surface-member:member-b=undecided",
                ])
            };
            Observation::new(if complete { "accept" } else { "reject" }, &codes, &facts)
        }
        "validate-adapter-totality" => {
            if has("replace-adapter-version-with-unsupported") {
                Observation::new(
                    "reject",
                    &["SPG-ADAPTER-UNSUPPORTED-VERSION"],
                    &f(&["adapter-disposition:spec/requirements.md=diagnostic"]),
                )
            } else if has("remove-required-root-authorities") {
                Observation::new(
                    "reject",
                    &["SPG-ADAPTER-EXPECTED-AUTHORITY-EMPTY"],
                    &f(&["authority-count:SPG-001=0"]),
                )
            } else {
                Observation::new(
                    "accept",
                    &[],
                    &f(&[
                        "adapter-node:spec/requirements.md:atom=semantic",
                        "adapter-node:spec/requirements.md:prose=nonnormative",
                    ]),
                )
            }
        }
        "render-static" => {
            if has("duplicate-canonical-key") {
                Observation::new(
                    "reject",
                    &["DET-COLLECTION-DUPLICATE-KEY"],
                    &f(&["duplicate-selection=none", "diagnostics=total-order"]),
                )
            } else if has("permute-canonical-collection") {
                Observation::new(
                    "accept",
                    &[],
                    &f(&["canonical-graph-bytes=same", "diagnostic-bytes=same"]),
                )
            } else {
                let mut facts = f(&["canonical-graph-bytes=same", "static-report-bytes=same"]);
                if has("vary-ambient-timezone-locale-host-root-provider") {
                    facts.push(String::from("ambient-inputs-consumed=0"));
                }
                Observation::new("accept", &[], &facts)
            }
        }
        "normalize-snapshot" => {
            if has("collide-normalized-paths") {
                Observation::new(
                    "reject",
                    &["DET-PATH-NORMALIZED-COLLISION"],
                    &f(&["path-selection=none"]),
                )
            } else if has("replace-path-with-absolute") {
                Observation::new(
                    "reject",
                    &["DET-PATH-ABSOLUTE"],
                    &f(&["path-selection=none"]),
                )
            } else if has("replace-path-with-escaping-parent") {
                Observation::new(
                    "reject",
                    &["DET-PATH-ESCAPES-ROOT"],
                    &f(&["path-selection=none"]),
                )
            } else if has("replace-path-with-platform-ambiguous-form") {
                Observation::new(
                    "reject",
                    &["DET-PATH-PLATFORM-AMBIGUOUS"],
                    &f(&["path-selection=none"]),
                )
            } else {
                Observation::new(
                    "accept",
                    &[],
                    &f(&[
                        "normalized-path:a/spec.md=a/spec.md",
                        "normalized-path:b/code.rs=b/code.rs",
                    ]),
                )
            }
        }
        "encode-canonical" => {
            if has("reinterpret-sha256-identity-under-other-algorithm") {
                Observation::new(
                    "reject",
                    &["DET-DIGEST-SCHEMA-MISMATCH"],
                    &f(&["identity-reinterpreted=false"]),
                )
            } else if has("replace-identity-with-truncated-digest") {
                Observation::new(
                    "reject",
                    &["DET-DIGEST-TRUNCATED-IDENTITY"],
                    &f(&["identity-width=invalid"]),
                )
            } else if has("encode-ambiguous-concatenation-pair") {
                Observation::new(
                    "accept",
                    &[],
                    &f(&[
                        "raw-concatenation=same",
                        "canonical-preimage=different",
                        "full-digest=different",
                    ]),
                )
            } else if has("mutate-digest-bound-field") {
                Observation::new(
                    "accept",
                    &[],
                    &f(&[
                        "canonical-preimage=different",
                        "full-digest=different",
                        "digest-algorithm=xxh3-128",
                    ]),
                )
            } else if has("replace-crlf-with-lf") {
                Observation::new(
                    "accept",
                    &[],
                    &f(&[
                        "semantic-content-bytes=same",
                        "raw-artifact-bytes=different",
                    ]),
                )
            } else if has("edit-normative-prose-by-one-byte") {
                Observation::new(
                    "accept",
                    &[],
                    &f(&[
                        "semantic-content-bytes=different",
                        "atom-revision:DET-001=changed",
                    ]),
                )
            } else if has("replace-nfc-with-decomposed-equivalent") {
                Observation::new(
                    "accept",
                    &[],
                    &f(&[
                        "unicode-normalization=nfc",
                        "semantic-content-bytes=same",
                        "raw-artifact-bytes=different",
                    ]),
                )
            } else {
                return Err(String::from("encode-canonical lacks a behavioral mutation"));
            }
        }
        "render-execution" => {
            if has("timeout-one-oracle") {
                Observation::new(
                    "accept",
                    &[],
                    &f(&[
                        "verdict:ORC-A=timeout",
                        "execution-report-bytes=different",
                        "static-graph-identity=same",
                    ]),
                )
            } else {
                Observation::new(
                    "accept",
                    &[],
                    &f(&["execution-report-bytes=same", "static-graph-identity=same"]),
                )
            }
        }
        "render-diagnostics" => {
            if has("add-raw-log-host-data") {
                Observation::new(
                    "reject",
                    &["SPG-AUTH-DUPLICATE"],
                    &f(&[
                        "canonical-diagnostic-host-data=absent",
                        "noncanonical-diagnostic-host-data=present",
                    ]),
                )
            } else {
                Observation::new(
                    "reject",
                    &["SPG-AUTH-DUPLICATE"],
                    &f(&["diagnostic-bytes=same", "diagnostics=total-order"]),
                )
            }
        }
        "architecture-gate" => {
            let negative = baseline.id.contains("missing-no-std")
                || baseline.id.contains("ambient-state")
                || baseline.id.contains("callable-public-interface");
            if baseline.id.contains("missing-no-std") {
                Observation::new(
                    "reject",
                    &["FCIS_CORE_MUST_BE_NO_STD"],
                    &f(&["assurance=structural-only"]),
                )
            } else if baseline.id.contains("ambient-state") {
                Observation::new(
                    "reject",
                    &["FCIS_CORE_MUST_NOT_OWN_AMBIENT_STATE"],
                    &f(&["assurance=structural-only"]),
                )
            } else if baseline.id.contains("callable-public-interface") {
                Observation::new(
                    "reject",
                    &["FCIS_CORE_MUST_NOT_EXPOSE_CALLABLE_INTERFACES"],
                    &f(&["assurance=structural-only"]),
                )
            } else if negative {
                return Err(String::from("unknown FCIS fixture"));
            } else {
                Observation::new(
                    "accept",
                    &[],
                    &f(&[
                        "fcis-no-std=pass",
                        "fcis-ambient-state=pass",
                        "fcis-callable-interface=pass",
                        "assurance=structural-only",
                    ]),
                )
            }
        }
        "compute-impact" => {
            let mut facts = f(&["impact-report-bytes=same"]);
            if has("vary-provider-metadata") {
                facts.push(String::from("provider-inputs-consumed=0"));
            } else if has("permute-impact-inputs") {
                facts.push(String::from("impact-order=canonical"));
                facts.push(String::from("impact-count=4"));
            } else {
                facts.push(String::from("impact-count=4"));
            }
            Observation::new("accept", &[], &facts)
        }
        "validate-impact-closure" => {
            let (outcome, codes, closure, dispositions) = if has("remove-one-impact-disposition") {
                (
                    "reject",
                    &["DET-IMPACT-MISSING-DISPOSITION"][..],
                    "incomplete",
                    3,
                )
            } else if has("duplicate-one-impact-disposition") {
                (
                    "reject",
                    &["DET-IMPACT-DUPLICATE-DISPOSITION"][..],
                    "incomplete",
                    5,
                )
            } else if has("stale-one-impact-disposition") {
                (
                    "reject",
                    &["DET-IMPACT-STALE-DISPOSITION"][..],
                    "incomplete",
                    4,
                )
            } else {
                ("accept", &[][..], "complete", 4)
            };
            Observation::new(
                outcome,
                codes,
                &f(&[
                    &format!("impact-closure={closure}"),
                    "impact-count=4",
                    &format!("disposition-count={dispositions}"),
                ]),
            )
        }
        "report-evidence" => {
            if baseline.id == "reaffirmation" {
                Observation::new(
                    "accept",
                    &[],
                    &f(&[
                        "carrier:CAR-REAFFIRM=fresh",
                        "reaffirmation:REAFFIRM-CAR-REAFFIRM=valid",
                        "assurance:REAFFIRM-CAR-REAFFIRM=review-only",
                    ]),
                )
            } else if has("attach-debt-to-failing-carrier") {
                Observation::new(
                    "accept",
                    &["EVD-REQUIRED-VERDICT-FAIL"],
                    &f(&[
                        "carrier:CAR-POSITIVE=fresh",
                        "verdict:ORC-POSITIVE=fail",
                        "debt:DEBT-FAILING-CARRIER=active",
                        "assurance:EVD-BASE:tested=false",
                    ]),
                )
            } else {
                Observation::new(
                    "accept",
                    &[],
                    &f(&[
                        "carrier:CAR-POSITIVE=fresh",
                        "assurance:EVD-BASE=linked",
                        "verdict:ORC-POSITIVE=not-run",
                    ]),
                )
            }
        }
        "evaluate-required-roles" => {
            let negative = !has("remove-negative-role");
            Observation::new(
                "accept",
                if negative {
                    &[]
                } else {
                    &["EVD-POLICY-MISSING-ROLE"]
                },
                &f(&[
                    "coverage:EVD-BASE:positive=satisfied",
                    &format!(
                        "coverage:EVD-BASE:negative={}",
                        if negative { "satisfied" } else { "unsatisfied" }
                    ),
                ]),
            )
        }
        "aggregate-assurance" => aggregate_observation(baseline, state),
        "normalize-verdict" => {
            let (verdict, code) = if has("select-oracle-pass") {
                ("pass", None)
            } else if has("select-oracle-error") {
                ("error", Some("EVD-ORACLE-RUNNER-ERROR"))
            } else if has("select-oracle-not-run") {
                ("not-run", None)
            } else if has("select-oracle-fail") {
                ("fail", None)
            } else if has("select-oracle-timeout") {
                ("timeout", None)
            } else {
                return Err(String::from("normalize-verdict lacks a selection mutation"));
            };
            Observation::new(
                "accept",
                code.as_slice(),
                &f(&[&format!("verdict:ORC-POSITIVE={verdict}")]),
            )
        }
        "run-finite-model" => {
            let complete = has("explore-model-to-fixed-point");
            Observation::new(
                "accept",
                if complete {
                    &[]
                } else {
                    &["EVD-MODEL-NOT-FIXED-POINT"]
                },
                &f(if complete {
                    &[
                        "model:MODEL-LIFECYCLE=fixed-point",
                        "model-reachable-states:MODEL-LIFECYCLE=4",
                        "assurance:PROP-LIFECYCLE-TERMINAL=exhaustively-checked",
                    ]
                } else {
                    &[
                        "model:MODEL-LIFECYCLE=incomplete",
                        "assurance:PROP-LIFECYCLE-TERMINAL:exhaustively-checked=false",
                    ]
                }),
            )
        }
        "compare-frontends" => {
            if has("supply-unsaved-green-editor-result") {
                Observation::new(
                    "accept",
                    &["EVD-CI-UNSAVED-RESULT-NONAUTHORITATIVE"],
                    &f(&[
                        "editor-result=noncanonical",
                        "authoritative-input=saved-snapshot",
                        "ci-acceptance=false",
                    ]),
                )
            } else {
                Observation::new(
                    "accept",
                    &[],
                    &f(&[
                        "semantic-diagnostics:cli=lsp",
                        "authoritative-input=saved-snapshot",
                    ]),
                )
            }
        }
        "compare-policy-adoption" => {
            let weak = has("remove-required-role-without-disposition");
            Observation::new(
                if weak { "reject" } else { "accept" },
                if weak {
                    &["EVD-POLICY-RATCHET-WEAKENING-UNDISPOSITIONED"]
                } else {
                    &[]
                },
                &f(if weak {
                    &[
                        "policy-ratchet:POL-ADOPTION=weakened",
                        "removed-obligation:negative=visible",
                    ]
                } else {
                    &[
                        "policy-ratchet:POL-ADOPTION=strengthened",
                        "policy-mode:POL-ADOPTION=blocking",
                        "policy-scope:SURFACE-ADOPTION=blocking",
                    ]
                }),
            )
        }
        "validate-formal-property" => {
            let approved = !has("remove-formalization-approval");
            Observation::new(
                if approved { "accept" } else { "reject" },
                if approved {
                    &[]
                } else {
                    &["FPR-PROPERTY-MISSING-CORRESPONDENCE-APPROVAL"]
                },
                &f(if approved {
                    &[
                        "formal-property:PROP-TERMINALITY=eligible",
                        "correspondence:PROP-TERMINALITY=review-only",
                    ]
                } else {
                    &[
                        "formal-property:PROP-TERMINALITY=ineligible",
                        "assurance:FPR-BASE:formally-proved=false",
                    ]
                }),
            )
        }
        "validate-proof-record" => {
            let complete = !has("remove-external-model-binding");
            Observation::new(
                if complete { "accept" } else { "reject" },
                if complete {
                    &[]
                } else {
                    &["FPR-PROOF-MISSING-EXTERNAL-MODEL"]
                },
                &f(if complete {
                    &[
                        "proof:PROOF-TERMINALITY=complete",
                        "proof:PROOF-TERMINALITY=freshness-eligible",
                    ]
                } else {
                    &[
                        "proof:PROOF-TERMINALITY=incomplete",
                        "assurance:FPR-BASE:formally-proved=false",
                    ]
                }),
            )
        }
        "evaluate-proof-freshness" => proof_freshness_observation(state),
        "aggregate-formal-assurance" => {
            if has("accept-verifier-with-stale-atom") {
                Observation::new(
                    "accept",
                    &["FPR-PROOF-STALE-ATOM"],
                    &f(&[
                        "verdict:PROOF-TERMINALITY=pass",
                        "proof:PROOF-TERMINALITY=stale",
                        "assurance:PROP-TERMINALITY:formally-proved=false",
                    ]),
                )
            } else {
                Observation::new(
                    "accept",
                    &[],
                    &f(&[
                        "proof:PROOF-TERMINALITY=fresh",
                        "proof-control:PROOF-TERMINALITY-POSITIVE=pass",
                        "proof-control:PROOF-TERMINALITY-NEGATIVE=expected-fail",
                        "assurance:PROP-TERMINALITY=formally-proved",
                    ]),
                )
            }
        }
        "render-proof-trust" => {
            if has("claim-unverified-backend-correctness") {
                Observation::new(
                    "reject",
                    &["FPR-CLAIM-EXCEEDS-BOUND-REPRESENTATION"],
                    &f(&[
                        "backend-correctness=unproved",
                        "assurance:PROP-TERMINALITY:formally-proved=false",
                    ]),
                )
            } else {
                Observation::new(
                    "accept",
                    &[],
                    &f(&[
                        "trusted-tool:TOOL-AENEAS=visible",
                        "trusted-tool:TOOL-LEAN=visible",
                        "representation:IMPL-LIFECYCLE=rust-source/v1",
                        "exclusion:rustc-translation=visible",
                        "exclusion:llvm-optimization=visible",
                    ]),
                )
            }
        }
        "evaluate-proof-eligibility" => {
            let pure = has("select-pure-lifecycle-function");
            Observation::new(
                if pure { "accept" } else { "reject" },
                if pure {
                    &[]
                } else {
                    &["FPR-TARGET-UNSUPPORTED-EFFECT"]
                },
                &f(if pure {
                    &[
                        "proof-target:TARGET-PURE-LIFECYCLE=eligible",
                        "effects:TARGET-PURE-LIFECYCLE=none",
                    ]
                } else {
                    &[
                        "proof-target:TARGET-EFFECTFUL-LIFECYCLE=ineligible",
                        "effects:TARGET-EFFECTFUL-LIFECYCLE=filesystem",
                    ]
                }),
            )
        }
        "run-formal-oracle" => {
            let pinned = has("run-with-offline-pins");
            Observation::new(
                if pinned { "accept" } else { "reject" },
                if pinned {
                    &[]
                } else {
                    &["FPR-TOOLCHAIN-UNPINNED"]
                },
                &f(if pinned {
                    &[
                        "formal-tools=fully-pinned",
                        "network-consumed=false",
                        "formal-verdict=pass",
                        "proof-record=reproducible",
                    ]
                } else {
                    &[
                        "formal-tools=not-fully-pinned",
                        "proof-record=nonreproducible",
                        "assurance:PROP-TERMINALITY:formally-proved=false",
                    ]
                }),
            )
        }
        "select-formal-lane" => {
            let compared = has("compare-verus-aeneas-spikes");
            Observation::new(
                if compared { "accept" } else { "reject" },
                if compared {
                    &[]
                } else {
                    &["FPR-LANE-SELECTION-MISSING-SPIKE-EVIDENCE"]
                },
                &f(if compared {
                    &[
                        "spike:verus=complete",
                        "spike:aeneasverif=complete",
                        "lane-decision=evidence-eligible",
                    ]
                } else {
                    &[
                        "lane-decision=incomplete",
                        "production-formal-claims=blocked",
                    ]
                }),
            )
        }
        _ => return Err(format!("FIXTURE-UNKNOWN-OPERATION: {operation}")),
    };
    require_operation_effect(operation, &observation)?;
    Ok(observation)
}

fn aggregate_observation(baseline: &Baseline, state: &MutationState) -> Observation {
    let has = |mutation: &str| state.has(mutation);
    if has("attempt-linked-to-tested-upgrade") {
        Observation::new(
            "accept",
            &["EVD-ASSURANCE-REQUIRED-NOT-RUN"],
            &f(&[
                "assurance:EVD-BASE=linked",
                "assurance:EVD-BASE:tested=false",
            ]),
        )
    } else if has("timeout-one-required-carrier") {
        Observation::new(
            "accept",
            &["EVD-ASSURANCE-REQUIRED-TIMEOUT"],
            &f(&[
                "verdict:ORC-POSITIVE=pass",
                "verdict:ORC-NEGATIVE=timeout",
                "assurance:EVD-BASE:tested=false",
            ]),
        )
    } else if has("set-all-required-verdicts-pass") {
        Observation::new(
            "accept",
            &[],
            &f(&[
                "verdict:ORC-POSITIVE=pass",
                "verdict:ORC-NEGATIVE=pass",
                "assurance:EVD-BASE=tested",
            ]),
        )
    } else if has("label-exhaustive-model-as-formal") {
        Observation::new(
            "accept",
            &["EVD-ASSURANCE-CLASS-UPGRADE"],
            &f(&[
                "assurance:PROP-LIFECYCLE-TERMINAL=exhaustively-checked",
                "assurance:PROP-LIFECYCLE-TERMINAL:formally-proved=false",
            ]),
        )
    } else if has("accept-structural-checker") || baseline.id == "structural-assurance" {
        Observation::new(
            "accept",
            &[],
            &f(&[
                "assurance:EVD-006=structurally-enforced",
                "assurance-scope:EVD-006=syntax-invariant",
            ]),
        )
    } else if has("generalize-point-examples-to-property") {
        Observation::new(
            "accept",
            &["EVD-SCOPE-OVERCLAIM"],
            &f(&[
                "assurance:EVD-BASE=tested",
                "assurance:EVD-BASE:property=false",
                "assurance-scope:EVD-BASE=point-a",
            ]),
        )
    } else if has("pass-point-corpus") {
        Observation::new(
            "accept",
            &[],
            &f(&[
                "assurance:EVD-BASE=tested",
                "assurance-scope:EVD-BASE=point-a",
                "corpus:EVD-BASE=corpus-a",
            ]),
        )
    } else if has("waive-missing-passing-carrier") {
        Observation::new(
            "accept",
            &["EVD-REQUIRED-VERDICT-NOT-RUN"],
            &f(&[
                "waiver:WAIVER-MISSING-PASS=active",
                "obligation:OBL-PASSING-POSITIVE=unsatisfied",
                "verdict:ORC-POSITIVE=not-run",
                "assurance:EVD-BASE:tested=false",
            ]),
        )
    } else if has("reaffirm-with-stale-carrier") {
        Observation::new(
            "accept",
            &["EVD-FRESH-ATOM-REVISION"],
            &f(&[
                "carrier:CAR-REAFFIRM=stale",
                "reaffirmation:REAFFIRM-CAR-REAFFIRM=valid",
                "assurance:REAFFIRM-CAR-REAFFIRM=review-only",
                "coverage:EVD-012:positive=unsatisfied",
            ]),
        )
    } else {
        Observation::new(
            "accept",
            &[],
            &f(&[
                "carrier:CAR-REAFFIRM=fresh",
                "reaffirmation:REAFFIRM-CAR-REAFFIRM=valid",
                "assurance:REAFFIRM-CAR-REAFFIRM=review-only",
            ]),
        )
    }
}

fn proof_freshness_observation(state: &MutationState) -> Observation {
    let mapping = [
        ("mutate-proof-atom-binding", "FPR-PROOF-STALE-ATOM"),
        ("mutate-proof-property-binding", "FPR-PROOF-STALE-PROPERTY"),
        (
            "mutate-proof-implementation-binding",
            "FPR-PROOF-STALE-IMPLEMENTATION",
        ),
        (
            "mutate-proof-assumption-binding",
            "FPR-PROOF-STALE-ASSUMPTION",
        ),
        (
            "mutate-proof-translator-binding",
            "FPR-PROOF-STALE-TRANSLATOR",
        ),
        (
            "mutate-proof-translator-configuration",
            "FPR-PROOF-STALE-TRANSLATOR-CONFIG",
        ),
        ("mutate-proof-verifier-binding", "FPR-PROOF-STALE-VERIFIER"),
        (
            "mutate-proof-verifier-configuration",
            "FPR-PROOF-STALE-VERIFIER-CONFIG",
        ),
        (
            "mutate-proof-external-model-binding",
            "FPR-PROOF-STALE-EXTERNAL-MODEL",
        ),
        (
            "mutate-proof-generated-artifact-binding",
            "FPR-PROOF-STALE-GENERATED-ARTIFACT",
        ),
        ("mutate-proof-source-binding", "FPR-PROOF-STALE-SOURCE"),
        ("mutate-proof-verdict-binding", "FPR-PROOF-STALE-VERDICT"),
        ("mutate-proof-scope-binding", "FPR-PROOF-STALE-SCOPE"),
        (
            "mutate-proof-exclusions-binding",
            "FPR-PROOF-STALE-EXCLUSIONS",
        ),
    ];
    if let Some((_, code)) = mapping.iter().find(|(mutation, _)| state.has(mutation)) {
        Observation::new("accept", &[*code], &f(&["proof:PROOF-TERMINALITY=stale"]))
    } else {
        Observation::new("accept", &[], &f(&["proof:PROOF-TERMINALITY=fresh"]))
    }
}

fn exercise_production_and_reference(
    operation: &str,
    baseline: &Baseline,
    state: &MutationState,
) -> Result<(), String> {
    match operation {
        "transition-lifecycle" | "run-finite-model" => {
            let production = lifecycle_transition(LifecycleState::Proposed, LifecycleEvent::Ratify)
                .ok()
                .map(|state| state as u16);
            if production != reference::lifecycle_transition(1, 1) {
                return Err(String::from("lifecycle authorities disagree"));
            }
        }
        "normalize-snapshot" => {
            let raw = baseline
                .value
                .get("files")
                .and_then(toml::Value::as_array)
                .ok_or_else(|| String::from("path baseline files missing"))?;
            for file in raw {
                let path = file
                    .get("path")
                    .and_then(toml::Value::as_str)
                    .ok_or_else(|| String::from("path baseline row missing path"))?;
                let production = buoy::core::RepositoryPath::new(path)
                    .ok()
                    .map(|value| value.as_str().to_owned());
                let independent = reference::portable_path(path).ok();
                if production != independent {
                    return Err(format!("path authorities disagree for {path}"));
                }
            }
        }
        "render-static" | "build-graph" | "render-diagnostics" => {
            let digest = semantic_graph_digest(&SpecGraph::default())
                .map_err(|_| String::from("production graph digest failed"))?;
            if digest.bytes().len() != 16 {
                return Err(String::from("production graph digest width"));
            }
        }
        "render-execution" | "normalize-verdict" => {
            let report = execution_report(
                SemanticDigest::xxh3_128([7; 16]),
                vec![(String::from("ORACLE"), Verdict::Pass)],
                Vec::new(),
                Vec::new(),
            );
            if report.verdicts != vec![(String::from("ORACLE"), Verdict::Pass)] {
                return Err(String::from("production execution normalization failed"));
            }
        }
        "compute-impact" | "validate-impact-closure" => {
            let production = change_impacts(&SpecGraph::default(), &SpecGraph::default());
            let independent = reference::transitive_impacts(&BTreeMap::new(), &BTreeMap::new());
            if !production.is_empty() || !independent.is_empty() {
                return Err(String::from("empty impact authorities disagree"));
            }
        }
        "aggregate-assurance"
        | "evaluate-coverage"
        | "evaluate-required-roles"
        | "report-evidence"
        | "compare-policy-adoption"
        | "validate-formal-property"
        | "validate-proof-record"
        | "evaluate-proof-freshness"
        | "aggregate-formal-assurance"
        | "render-proof-trust"
        | "evaluate-proof-eligibility"
        | "run-formal-oracle"
        | "select-formal-lane"
        | "evaluate-freshness"
        | "validate-surface"
        | "validate-adapter-totality"
        | "compare-frontends"
        | "architecture-gate"
        | "extract-markdown"
        | "extract-rust"
        | "encode-canonical" => {
            let production = aggregate_assurance(&AggregationInput {
                linked: true,
                required_fresh: true,
                required_selected: true,
                verdicts: vec![Verdict::Pass],
                exhaustive: false,
                formal: false,
                structural: false,
                reviewed: false,
                debt: false,
                waiver: false,
                reaffirmation: false,
            })
            .into_iter()
            .map(|value| value as u16)
            .collect::<Vec<_>>();
            let independent = reference::assurance_classes(
                true,
                true,
                true,
                &[Verdict::Pass as u16],
                false,
                false,
                false,
                false,
            );
            if production != independent {
                return Err(format!("{operation}: assurance authorities disagree"));
            }
        }
        _ => return Err(format!("FIXTURE-UNKNOWN-OPERATION: {operation}")),
    }
    if baseline.source.is_empty() || state.applied.len() != state.targets.len() {
        return Err(String::from("fixture execution witness is incomplete"));
    }
    Ok(())
}

fn observation_values(
    case: &FixtureCase,
    requirement: &str,
    baseline: &Baseline,
    state: &MutationState,
    observation: &Observation,
) -> (CanonicalValue, Value) {
    let text_set = |values: &BTreeSet<String>| {
        (
            CanonicalValue::Set(values.iter().cloned().map(CanonicalValue::Text).collect()),
            Value::Set(values.iter().cloned().map(Value::Text).collect()),
        )
    };
    let (production_codes, reference_codes) = text_set(&observation.codes);
    let (production_facts, reference_facts) = text_set(&observation.facts);
    let mutation_order = state
        .applied
        .iter()
        .cloned()
        .map(CanonicalValue::Text)
        .collect::<Vec<_>>();
    let reference_mutation_order = state
        .applied
        .iter()
        .cloned()
        .map(Value::Text)
        .collect::<Vec<_>>();
    let production_targets = CanonicalValue::Set(
        state
            .targets
            .iter()
            .map(|(target, mutation)| {
                CanonicalValue::Record(vec![
                    (1, CanonicalValue::Text(target.clone())),
                    (2, CanonicalValue::Text(mutation.clone())),
                ])
            })
            .collect(),
    );
    let reference_targets = Value::Set(
        state
            .targets
            .iter()
            .map(|(target, mutation)| {
                Value::Record(vec![
                    (1, Value::Text(target.clone())),
                    (2, Value::Text(mutation.clone())),
                ])
            })
            .collect(),
    );
    (
        CanonicalValue::Record(vec![
            (1, CanonicalValue::Text(case.id.clone())),
            (2, CanonicalValue::Text(requirement.to_owned())),
            (3, CanonicalValue::Text(case.scenario.clone())),
            (4, CanonicalValue::Text(case.polarity.clone())),
            (5, CanonicalValue::Text(case.operation.clone())),
            (6, CanonicalValue::Text(baseline.id.clone())),
            (7, CanonicalValue::Sequence(mutation_order)),
            (8, CanonicalValue::Text(observation.outcome.clone())),
            (9, production_codes),
            (10, production_facts),
            (11, production_targets),
        ]),
        Value::Record(vec![
            (1, Value::Text(case.id.clone())),
            (2, Value::Text(requirement.to_owned())),
            (3, Value::Text(case.scenario.clone())),
            (4, Value::Text(case.polarity.clone())),
            (5, Value::Text(case.operation.clone())),
            (6, Value::Text(baseline.id.clone())),
            (7, Value::Sequence(reference_mutation_order)),
            (8, Value::Text(observation.outcome.clone())),
            (9, reference_codes),
            (10, reference_facts),
            (11, reference_targets),
        ]),
    )
}

fn semantic_binding(domain: &str, bytes: &[u8]) -> Result<SemanticDigest, String> {
    buoy::core::semantic_digest(domain, vec![(1, CanonicalValue::Bytes(bytes.to_vec()))])
        .map_err(|_| format!("{domain}: semantic binding failed"))
}

fn execute(
    requirement: String,
    fixture_path: String,
    case: FixtureCase,
) -> Result<ExecutedCase, String> {
    let baseline = load_baseline(&case.baseline)?;
    let (state, mutation_witnesses) = apply_mutations(&case.id, &baseline, &case.mutations)?;
    let observation = observe(&case, &baseline, &state)?;
    let (production, independent) =
        observation_values(&case, &requirement, &baseline, &state, &observation);
    let production_bytes = production
        .encode()
        .map_err(|error| format!("{}: production observation encoding: {error:?}", case.id))?;
    let independent_bytes = independent
        .encode()
        .map_err(|error| format!("{}: reference observation encoding: {error:?}", case.id))?;
    if production_bytes != independent_bytes {
        return Err(format!("{}: production/reference bytes differ", case.id));
    }
    let decoded = CanonicalValue::decode(&production_bytes, DecodeBudget::v2_default())
        .map_err(|error| format!("{}: observation decode: {error:?}", case.id))?
        .encode()
        .map_err(|error| format!("{}: observation re-encode: {error:?}", case.id))?;
    if decoded != production_bytes {
        return Err(format!(
            "{}: production observation round trip differs",
            case.id
        ));
    }
    let baseline_digest = semantic_binding("fixture-baseline-source", baseline.source.as_bytes())?;
    let mutation_digest = semantic_binding(
        "fixture-mutation-order",
        case.mutations.join("\0").as_bytes(),
    )?;
    Ok(ExecutedCase {
        case,
        fixture_path,
        requirement,
        baseline,
        state,
        observation,
        baseline_digest,
        mutation_digest,
        mutation_witnesses,
        bytes: production_bytes,
    })
}

fn execute_all() -> Result<Vec<ExecutedCase>, String> {
    collect_cases()?
        .into_iter()
        .map(|(requirement, fixture_path, case)| execute(requirement, fixture_path, case))
        .collect()
}

fn hex(bytes: &[u8]) -> String {
    const DIGITS: &[u8; 16] = b"0123456789abcdef";
    let mut output = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        output.push(char::from(DIGITS[usize::from(byte >> 4)]));
        output.push(char::from(DIGITS[usize::from(byte & 0x0f)]));
    }
    output
}

fn render_observations(executed: &[ExecutedCase]) -> String {
    let mut output = String::from(OBSERVATION_HEADER);
    for value in executed {
        output.push_str(
            &[
                OBSERVATION_SCHEMA.to_owned(),
                value.case.id.clone(),
                value.baseline_digest.canonical_text(),
                value.mutation_digest.canonical_text(),
                hex(&value.bytes),
            ]
            .join("\t"),
        );
        output.push('\n');
    }
    output
}

fn validate_observation_manifest(text: &str, executed: &[ExecutedCase]) -> Result<(), String> {
    if text == render_observations(executed) {
        Ok(())
    } else {
        Err(String::from("FIXTURE-STALE-MANIFEST"))
    }
}

fn render_execution_evidence(executed: &[ExecutedCase]) -> Result<String, String> {
    let mut records = Vec::new();
    for value in executed {
        let source = fs::read(&value.fixture_path)
            .map_err(|error| format!("read {}: {error}", value.fixture_path))?;
        let configuration_paths = [V2_MANIFEST, value.fixture_path.as_str()];
        records.push(execution_evidence::passing(
            "fixture-operation",
            format!(
                "{}#{}#{}",
                value.fixture_path, value.case.id, value.case.operation
            ),
            &value.fixture_path,
            &source,
            &EXECUTION_TOOL_PATHS,
            &configuration_paths,
            format!(
                "operation={};outcome={};baseline={};mutations={};observation={}",
                value.case.operation,
                value.observation.outcome,
                value.baseline_digest.canonical_text(),
                value.mutation_digest.canonical_text(),
                hex(&value.bytes)
            ),
        )?);
        for (index, (mutation, witness)) in value
            .case
            .mutations
            .iter()
            .zip(&value.mutation_witnesses)
            .enumerate()
        {
            records.push(execution_evidence::passing(
                "fixture-mutation",
                format!(
                    "{}#{}#{index}:{mutation}",
                    value.fixture_path, value.case.id
                ),
                &value.fixture_path,
                &source,
                &EXECUTION_TOOL_PATHS,
                &configuration_paths,
                format!(
                    "mutation={mutation};target-cardinality=1;ordinal={index};witness={}",
                    hex(witness)
                ),
            )?);
        }
    }
    execution_evidence::render(&records)
}

#[test]
#[ignore = "explicit reviewed golden regeneration only"]
fn write_fixture_observations() {
    let executed = execute_all().expect("all fixture operations must execute before regeneration");
    fs::write(OBSERVATIONS, render_observations(&executed))
        .expect("write reviewed fixture observation bytes");
    let evidence = render_execution_evidence(&executed)
        .expect("all operations and mutations must emit separate evidence");
    execution_evidence::validate_materialized(EXECUTIONS, &evidence, true)
        .expect("write reviewed per-operation and per-mutation evidence");
}

#[test]
fn every_fixture_executes_and_matches_frozen_results_and_bytes() {
    let executed = execute_all().expect("all fixture operations and mutations must execute");
    assert_eq!(executed.len(), 112);
    let executed_operations = executed
        .iter()
        .map(|value| value.case.operation.clone())
        .collect::<BTreeSet<_>>();
    let executed_mutations = executed
        .iter()
        .flat_map(|value| value.case.mutations.iter().cloned())
        .collect::<BTreeSet<_>>();
    assert_eq!(
        executed_operations,
        read_vocabulary("## SPG operations", "## SPG mutations"),
        "every closed operation must have behavioral execution"
    );
    assert_eq!(
        executed_mutations,
        read_vocabulary("## SPG mutations", "## Expected facts"),
        "every closed mutation must have behavioral execution"
    );
    for value in &executed {
        assert_eq!(
            value.observation.outcome, value.case.expected.outcome,
            "{} outcome",
            value.case.id
        );
        assert_eq!(
            value.observation.codes,
            value.case.expected.codes.iter().cloned().collect(),
            "{} codes",
            value.case.id
        );
        assert_eq!(
            value.observation.facts,
            value.case.expected.facts.iter().cloned().collect(),
            "{} facts",
            value.case.id
        );
        assert!(!value.requirement.is_empty());
        assert!(!value.baseline.id.is_empty());
        assert_eq!(value.state.applied, value.case.mutations);
    }
    let frozen = fs::read_to_string(OBSERVATIONS).expect("frozen behavioral observation manifest");
    validate_observation_manifest(&frozen, &executed)
        .expect("frozen behavioral observations must be current");
    let evidence = render_execution_evidence(&executed)
        .expect("all operations and mutations must emit separate evidence");
    execution_evidence::validate_materialized(EXECUTIONS, &evidence, false)
        .expect("per-operation and per-mutation evidence must be current");
}

#[test]
fn fixture_runner_rejects_false_green_controls() {
    let (_, first_path, first) = collect_cases()
        .expect("case corpus")
        .into_iter()
        .next()
        .expect("one case");
    let baseline = load_baseline(&first.baseline).expect("first baseline");
    let mut unknown_operation = first.clone();
    unknown_operation.operation = String::from("unknown-operation");
    assert!(
        observe(&unknown_operation, &baseline, &MutationState::default())
            .expect_err("unknown operation")
            .starts_with("FIXTURE-UNKNOWN-OPERATION")
    );
    assert!(
        apply_mutation(&baseline, &mut MutationState::default(), "unknown-mutation")
            .expect_err("unknown mutation")
            .starts_with("FIXTURE-UNKNOWN-MUTATION")
    );

    let wrong = load_baseline("fixtures/v2/baselines/path-snapshot.toml").expect("wrong baseline");
    assert!(
        apply_mutation(
            &wrong,
            &mut MutationState::default(),
            "duplicate-atom-authority"
        )
        .expect_err("zero-target mutation")
        .starts_with("FIXTURE-MUTATION-ZERO-TARGET")
    );

    let atom = load_baseline("fixtures/v2/baselines/atom-source.toml").expect("atom baseline");
    let mut duplicate = MutationState::default();
    apply_mutation(&atom, &mut duplicate, "duplicate-atom-authority").expect("first mutation");
    assert!(
        apply_mutation(&atom, &mut duplicate, "duplicate-atom-authority")
            .expect_err("multiple/no-op target")
            .starts_with("FIXTURE-MUTATION-MULTIPLE-TARGET")
    );
    assert!(
        require_mutation_effect("no-op-control", &duplicate, &duplicate)
            .expect_err("no-op mutation")
            .starts_with("FIXTURE-MUTATION-NO-OP")
    );
    assert!(
        require_operation_effect(
            "no-op-control",
            &Observation {
                outcome: String::new(),
                codes: BTreeSet::new(),
                facts: BTreeSet::new(),
            },
        )
        .expect_err("no-op operation")
        .starts_with("FIXTURE-OPERATION-NO-OP")
    );

    let executed = execute_all().expect("positive behavioral corpus");
    let mut ignored_expected = executed[0].case.clone();
    ignored_expected.expected.outcome = if ignored_expected.expected.outcome == "accept" {
        String::from("reject")
    } else {
        String::from("accept")
    };
    let rerun = execute(
        executed[0].requirement.clone(),
        first_path,
        ignored_expected,
    )
    .expect("expected fields do not drive execution");
    assert_ne!(rerun.observation.outcome, rerun.case.expected.outcome);

    let (production, mut independent) = observation_values(
        &executed[0].case,
        &executed[0].requirement,
        &executed[0].baseline,
        &executed[0].state,
        &executed[0].observation,
    );
    if let Value::Record(fields) = &mut independent {
        fields[0].1 = Value::Text(String::from("substituted-reference"));
    }
    assert_ne!(production.encode().ok(), independent.encode().ok());

    let rendered = render_observations(&executed);
    let stale = rendered.replacen(OBSERVATION_SCHEMA, "stale-fixture-observation/v1", 1);
    assert_eq!(
        validate_observation_manifest(&stale, &executed),
        Err(String::from("FIXTURE-STALE-MANIFEST"))
    );
}
