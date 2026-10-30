//! Dependency-free frozen-v3 schema inventory validator.

#![allow(clippy::cast_possible_truncation, clippy::too_many_lines)]

#[path = "digest_binding.rs"]
mod content_digest;
mod execution_evidence;

use execution_evidence::Record as ExecutionRecord;
use std::collections::{BTreeMap, BTreeSet};
#[cfg(not(test))]
use std::env;
use std::fmt::Write as _;
use std::fs;
#[cfg(all(test, feature = "coverage-controls"))]
use std::sync::atomic::{AtomicU8, Ordering};

const DEFAULT_INVENTORY: &str = "schema/v3/inventory.tsv";
const DEFAULT_CONSTRUCTION_FIXTURES: &str =
    "fixtures/hardening/v3/schema-construction-fixtures.tsv";
const DEFAULT_STATE_FIXTURES: &str = "fixtures/hardening/v3/schema-state-fixtures.tsv";
const DEFAULT_IMPACT_FIXTURES: &str = "fixtures/hardening/v3/impact-edge-fixtures.tsv";
const DEFAULT_POLICY_FIXTURES: &str = "fixtures/hardening/v3/policy-ratchet-fixtures.tsv";
const DEFAULT_DISPOSITION_FIXTURES: &str =
    "fixtures/hardening/v3/disposition-protocol-fixtures.tsv";
const DEFAULT_CANONICAL_VECTORS: &str = "fixtures/hardening/v3/canonical-vectors.tsv";
const EXECUTION_EVIDENCE: &str = "assurance/v1/schema-fixture-executions.tsv";
const EXECUTION_TOOL_PATHS: [&str; 3] = [
    "tools/check_schema_inventory.rs",
    "tools/execution_evidence.rs",
    "tools/digest_binding.rs",
];
const CANONICAL_VECTOR_HEADER: &str = "schema\tkind\tid\tcanonical_hex\tsemantic_digest";
const IMPLEMENTATION_SOURCES: [&str; 9] = [
    "crates/buoy-core/src/value.rs",
    "crates/buoy-core/src/identifier.rs",
    "crates/buoy-core/src/model.rs",
    "crates/buoy-core/src/attachment.rs",
    "crates/buoy-core/src/mount.rs",
    "crates/buoy-core/src/algorithms.rs",
    "crates/buoy-core/src/report.rs",
    "src/repository.rs",
    "src/migration.rs",
];
const EXECUTION_CONFIGURATION_PATHS: [&str; 16] = [
    DEFAULT_INVENTORY,
    DEFAULT_CONSTRUCTION_FIXTURES,
    DEFAULT_STATE_FIXTURES,
    DEFAULT_IMPACT_FIXTURES,
    DEFAULT_POLICY_FIXTURES,
    DEFAULT_DISPOSITION_FIXTURES,
    DEFAULT_CANONICAL_VECTORS,
    "crates/buoy-core/src/value.rs",
    "crates/buoy-core/src/identifier.rs",
    "crates/buoy-core/src/model.rs",
    "crates/buoy-core/src/attachment.rs",
    "crates/buoy-core/src/mount.rs",
    "crates/buoy-core/src/algorithms.rs",
    "crates/buoy-core/src/report.rs",
    "src/repository.rs",
    "src/migration.rs",
];
const ENUM_API_BINDINGS: [(&str, &str); 36] = [
    ("digest-algorithm", "pub enum DigestAlgorithm"),
    ("object-kind", "pub enum ObjectKind"),
    ("atom-kind", "pub enum AtomKind"),
    ("live-lifecycle", "pub enum LifecycleState"),
    ("terminal-lifecycle", "pub enum LifecycleState"),
    ("evidence-role", "pub enum EvidenceRole"),
    ("enforcement-mode", "pub enum EnforcementMode"),
    ("policy-lifecycle", "pub enum PolicyLifecycle"),
    ("debt-lifecycle", "pub enum DebtLifecycle"),
    ("waiver-lifecycle", "pub enum WaiverLifecycle"),
    ("surface-disposition", "pub enum SurfaceDispositionKind"),
    ("navigation-kind", "pub enum NavigationKind"),
    ("external-trace-relation", "pub enum ExternalTraceRelation"),
    ("source-edge-kind", "pub enum SourceEdgeKind"),
    ("artifact-binding-kind", "pub enum ArtifactBindingKind"),
    ("derivation-relation", "pub enum DerivationRelation"),
    ("oracle-kind", "pub enum OracleKind"),
    ("verdict", "pub enum Verdict"),
    ("source-role", "pub enum SourceRole"),
    ("change-disposition", "pub enum ChangeDispositionKind"),
    ("obligation-kind", "pub enum ObligationKind"),
    ("assurance-class", "pub enum AssuranceClass"),
    ("validity", "pub enum Validity"),
    ("completeness", "pub enum Completeness"),
    ("freshness", "pub enum Freshness"),
    ("satisfaction", "pub enum Satisfaction"),
    ("selection", "pub enum Selection"),
    ("closure", "pub enum Closure"),
    ("consistency", "pub enum Consistency"),
    ("diagnostic-severity", "pub enum DiagnosticSeverity"),
    ("impact-distance", "pub enum ImpactDistance"),
    ("entry-kind", "pub enum EntryKind"),
    ("authority-expectation", "pub enum AuthorityExpectation"),
    ("governance-lifecycle", "pub enum GovernanceLifecycle"),
    ("lifecycle", "pub enum LifecycleState"),
    ("id-allocation-class", "pub enum IdAllocationClass"),
];
const OBJECT_API_BINDINGS: [(&str, &str); 20] = [
    ("atom", "pub struct Atom"),
    ("tombstone", "pub struct Tombstone"),
    ("decision-surface", "pub struct DecisionSurface"),
    ("evidence-carrier", "pub struct EvidenceCarrier"),
    ("implementation-link", "pub struct ImplementationLink"),
    ("derivation", "pub struct Derivation"),
    ("coverage-policy", "pub struct CoveragePolicy"),
    ("debt-record", "pub struct DebtRecord"),
    ("waiver", "pub struct Waiver"),
    ("oracle", "pub struct Oracle"),
    ("finite-model", "pub struct FiniteModel"),
    ("formal-property", "pub struct FormalProperty"),
    ("assumption", "pub struct Assumption"),
    ("proof-record", "pub struct ProofRecord"),
    ("parsed-item", "pub struct ParsedItemIdentity"),
    ("reaffirmation", "pub struct Reaffirmation"),
    ("change-manifest", "pub struct ChangeManifest"),
    ("change-disposition", "pub struct ChangeDisposition"),
    ("external-trace-link", "pub struct ExternalTraceLink"),
    ("implementation-binding", "pub struct ImplementationBinding"),
];
const NESTED_API_BINDINGS: [(&str, &str); 24] = [
    ("object-reference", "pub struct ObjectRef"),
    ("atom-revision-reference", "pub struct AtomRevisionRef"),
    ("source-reference", "pub struct SourceRef"),
    ("approval-binding", "pub struct ApprovalBinding"),
    ("tool-identity", "pub struct ToolIdentity"),
    ("typed-payload", "pub struct TypedPayload"),
    ("typed-selector", "pub struct TypedSelector"),
    ("logical-mount", "pub struct LogicalMount"),
    ("mounted-selector", "pub struct Selector"),
    (
        "artifact-binding-attachment",
        "pub struct ArtifactBindingAttachment",
    ),
    ("id-namespace", "pub struct IdNamespace"),
    ("id-allocation", "pub struct IdAllocation"),
    ("approval-authority", "pub struct ApprovalAuthority"),
    (
        "rejected-entry-observation",
        "pub struct RejectedEntryObservation",
    ),
    ("policy-ratchet-impact", "pub struct PolicyRatchetImpact"),
    ("environment-binding", "pub struct EnvironmentBinding"),
    ("snapshot-budgets", "pub struct SnapshotBudgets"),
    ("oracle-command", "pub struct OracleCommand"),
    ("artifact-entry", "pub struct ArtifactEntry"),
    ("source-item-kind", "SourceItemKind,"),
    ("source-child-reference", "pub struct SourceChildRef"),
    ("source-edge", "pub struct SourceEdge"),
    ("source-budgets", "pub struct SourceBudgets"),
    ("external-trace-target", "pub struct ExternalTraceTarget"),
];
const DOMAIN_API_BINDINGS: [(&str, &str); 49] = [
    ("repository-execution-config", "impl RepositoryConfig"),
    ("buoy/repository-config/v3", "impl RepositoryConfig"),
    ("raw-artifact", "pub fn raw_artifact_digest_with"),
    ("repository-config", "pub struct SemanticRepositoryConfig"),
    ("repository-snapshot", "impl RepositorySnapshot"),
    ("snapshot-file", "impl SnapshotFile"),
    ("schema-tool-lock", "pub struct SchemaToolLock"),
    ("atom-revision", "impl Atom"),
    ("atom-object", "impl Atom"),
    ("tombstone", "impl Tombstone"),
    ("id-registry", "impl IdRegistry"),
    ("decision-surface-domain", "impl DecisionSurfaceDomain"),
    ("surface-disposition", "impl SurfaceDisposition"),
    ("decision-surface", "impl DecisionSurface"),
    ("evidence-carrier", "impl EvidenceCarrier"),
    ("parsed-item-identity", "impl ParsedItemIdentity"),
    ("source-node", "impl SourceNode"),
    ("source-graph", "impl SourceGraph"),
    ("implementation-link", "impl ImplementationLink"),
    (
        "buoy/implementation-binding/v1",
        "impl ImplementationBinding",
    ),
    (
        "buoy/binding-attachment/v1",
        "impl ArtifactBindingAttachment",
    ),
    (
        "buoy/binding-attachment-key/v1",
        "pub fn binding_attachment_key",
    ),
    ("external-trace-link", "impl ExternalTraceLink"),
    ("derivation", "impl Derivation"),
    ("artifact-binding", "impl ArtifactBinding"),
    ("artifact-index", "impl ArtifactIndex"),
    ("coverage-policy-revision", "impl CoveragePolicyRevision"),
    ("coverage-policy-object", "impl CoveragePolicy"),
    ("debt-record", "impl DebtRecord"),
    ("waiver", "impl Waiver"),
    ("reaffirmation", "impl Reaffirmation"),
    ("obligation-reference", "impl ObligationRef"),
    ("oracle", "impl Oracle"),
    ("normalized-verdict", "impl NormalizedVerdict"),
    ("verdict-set", "impl CanonicalExecutionReport"),
    ("assumption", "impl Assumption"),
    ("finite-model", "impl FiniteModel"),
    ("formal-property", "impl FormalProperty"),
    ("proof-record", "impl ProofRecord"),
    ("source-map-entry", "impl SourceMapEntry"),
    ("source-map", "impl SourceMap"),
    ("spec-graph-semantic", "pub fn semantic_graph_digest_with"),
    ("spec-graph-complete", "pub fn complete_graph_digest_with"),
    ("change-manifest", "impl ChangeManifest"),
    ("change-disposition", "impl ChangeDisposition"),
    ("impact-set", "pub fn impact_set_digest_with"),
    ("static-report", "impl CanonicalStaticReport"),
    ("execution-report", "impl CanonicalExecutionReport"),
    ("impact-report", "impl CanonicalImpactReport"),
];
const REPORT_API_BINDINGS: [(&str, &str); 11] = [
    ("authority-fact", "impl AuthorityFact"),
    ("surface-fact", "impl SurfaceFact"),
    ("registration-fact", "impl RegistrationFact"),
    ("policy-fact", "impl PolicyFact"),
    ("freshness-fact", "impl FreshnessFact"),
    ("debt-waiver-fact", "impl DebtWaiverFact"),
    ("derivation-fact", "impl DerivationFact"),
    ("proof-fact", "impl ProofFact"),
    ("evidence-fact", "impl EvidenceFact"),
    ("disposition-closure-fact", "impl DispositionClosureFact"),
    ("diagnostic", "impl CanonicalDiagnostic"),
];
const HEADER: &str = "category\tparent\tname\ttag\tshape\tcardinality\tcollection\trule";
const CONSTRUCTION_FIXTURE_HEADER: &str = "schema\tcase_id\tpolarity\ttarget_category\ttarget_parent\ttarget_name\toperation\tinput_form\tmutation\texpected_outcome\texpected_diagnostic";
const CONSTRUCTION_FIXTURE_SCHEMA: &str = "buoy-schema-construction-fixtures/v2";
const STATE_FIXTURE_HEADER: &str = "schema\tcase_id\tpolarity\tsubject\toperation\tbase_state\thead_state\tmutation\texpected_outcome\texpected_diagnostic\texpected_facts";
const STATE_FIXTURE_SCHEMA: &str = "buoy-schema-state-fixtures/v2";
const IMPACT_FIXTURE_HEADER: &str = "schema\tcase_id\tpolarity\tedge_id\tsource\ttarget\tbase_graph\thead_graph\tchanged_target\tchain_length\tmutation\texpected_outcome\texpected_diagnostic\texpected_impacts";
const IMPACT_FIXTURE_SCHEMA: &str = "buoy-impact-edge-fixtures/v2";
const POLICY_FIXTURE_HEADER: &str = "schema\tcase_id\tpolarity\tdimension\tbase_state\thead_state\tmutation\texpected_classification\texpected_outcome\texpected_diagnostic\texpected_facts";
const POLICY_FIXTURE_SCHEMA: &str = "buoy-policy-ratchet-fixtures/v2";
const DISPOSITION_FIXTURE_HEADER: &str = "schema\tcase_id\tpolarity\tdisposition\tmutation\tbase_binding\thead_binding\texpected_outcome\texpected_diagnostic\texpected_facts";
const DISPOSITION_FIXTURE_SCHEMA: &str = "buoy-disposition-protocol-fixtures/v2";
const REQUIRED_DISPOSITION_KINDS: [&str; 4] = ["reaffirmed", "retired", "revised", "waived"];
#[cfg(all(test, feature = "coverage-controls"))]
static FORCED_FIXTURE_FAILURE: AtomicU8 = AtomicU8::new(0);

const REQUIRED_DISPOSITION_MUTATIONS: [&str; 8] = [
    "applicability",
    "approval",
    "base-digest",
    "debt",
    "head-digest",
    "record",
    "replacement",
    "waiver",
];
const REQUIRED_POLICY_DIMENSIONS: [&str; 10] = [
    "authority",
    "derivation",
    "enforcement-mode",
    "evidence-role",
    "lineage",
    "retirement",
    "scope",
    "supersession",
    "surface-evolution",
    "waiver-class",
];
const REQUIRED_POLICY_CASES: [&str; 20] = [
    "policy-authority-hidden-weakening",
    "policy-authority-strengthening",
    "policy-derivation-hidden-weakening",
    "policy-derivation-strengthening",
    "policy-enforcement-mode-hidden-weakening",
    "policy-enforcement-mode-strengthening",
    "policy-evidence-role-hidden-weakening",
    "policy-evidence-role-strengthening",
    "policy-lineage-hidden-weakening",
    "policy-lineage-retained",
    "policy-retirement-complete",
    "policy-retirement-missing-protocol",
    "policy-scope-hidden-weakening",
    "policy-scope-strengthening",
    "policy-supersession-complete",
    "policy-supersession-missing-replacement",
    "policy-surface-evolution-hidden-weakening",
    "policy-surface-evolution-strengthening",
    "policy-waiver-class-hidden-weakening",
    "policy-waiver-class-strengthening",
];
const REQUIRED_STATE_SUBJECTS: [&str; 9] = [
    "approval-binding",
    "artifact-index",
    "id-namespace",
    "id-registry",
    "parsed-item-identity",
    "source-map",
    "terminal-debt",
    "terminal-policy",
    "terminal-waiver",
];
const REQUIRED_STATE_CASES: [&str; 39] = [
    "approval-exact-current",
    "approval-stale-record",
    "approval-unauthorized-action",
    "approval-unknown-authority",
    "artifact-index-duplicate-path",
    "artifact-index-exact-current",
    "artifact-index-head-bytes-stale",
    "artifact-index-missing-selected-path",
    "id-namespace-known-allocation",
    "id-namespace-unknown-allocation",
    "id-namespace-wrong-prefix",
    "id-registry-append-allocation",
    "id-registry-duplicate-allocation",
    "id-registry-remove-allocation",
    "id-registry-reuse-terminal-id",
    "parsed-item-invalid-enclosing-item",
    "parsed-item-semantic-edit-old-digest",
    "parsed-item-semantic-preserving-move",
    "source-map-dangling-object",
    "source-map-duplicate-role",
    "source-map-moved-source-current",
    "source-map-out-of-bounds-span",
    "terminal-debt-discharged-complete",
    "terminal-debt-discharged-missing-evidence",
    "terminal-debt-removed",
    "terminal-debt-superseded-complete",
    "terminal-debt-superseded-missing-replacement",
    "terminal-policy-retired-complete",
    "terminal-policy-retired-missing-approval",
    "terminal-policy-removed",
    "terminal-policy-superseded-complete",
    "terminal-policy-superseded-missing-replacement",
    "terminal-waiver-discharged-complete",
    "terminal-waiver-discharged-missing-evidence",
    "terminal-waiver-removed",
    "terminal-waiver-revoked-complete",
    "terminal-waiver-revoked-still-authorizes",
    "terminal-waiver-superseded-complete",
    "terminal-waiver-superseded-missing-replacement",
];
const CATEGORIES: [&str; 12] = [
    "schema",
    "collection-rule",
    "type-tag",
    "enum",
    "enum-variant",
    "object",
    "nested",
    "domain",
    "field",
    "report-fact",
    "reference-kind",
    "impact-edge",
];
const EXTRA_CATEGORIES: [&str; 2] = ["lifecycle-transition", "diagnostic-family"];
const REQUIRED_DOMAINS: [&str; 41] = [
    "raw-artifact",
    "repository-config",
    "repository-snapshot",
    "snapshot-file",
    "schema-tool-lock",
    "atom-revision",
    "atom-object",
    "tombstone",
    "id-registry",
    "decision-surface-domain",
    "surface-disposition",
    "decision-surface",
    "evidence-carrier",
    "parsed-item-identity",
    "source-node",
    "source-graph",
    "implementation-link",
    "buoy/implementation-binding/v1",
    "external-trace-link",
    "derivation",
    "artifact-binding",
    "coverage-policy-revision",
    "coverage-policy-object",
    "debt-record",
    "waiver",
    "reaffirmation",
    "obligation-reference",
    "oracle",
    "normalized-verdict",
    "verdict-set",
    "assumption",
    "finite-model",
    "formal-property",
    "proof-record",
    "source-map-entry",
    "source-map",
    "spec-graph-semantic",
    "spec-graph-complete",
    "static-report",
    "execution-report",
    "change-manifest",
];
const ADDITIONAL_DOMAINS: [&str; 4] = [
    "artifact-index",
    "change-disposition",
    "impact-set",
    "impact-report",
];
const REQUIRED_REPORT_FACTS: [&str; 11] = [
    "authority-fact",
    "surface-fact",
    "registration-fact",
    "policy-fact",
    "freshness-fact",
    "debt-waiver-fact",
    "derivation-fact",
    "proof-fact",
    "evidence-fact",
    "disposition-closure-fact",
    "diagnostic",
];
const REQUIRED_DIAGNOSTIC_FAMILIES: [&str; 27] = [
    "SCF-APPROVAL-*",
    "SCF-ARTIFACT-*",
    "SCF-CONSTRUCT-*",
    "SCF-LIFECYCLE-*",
    "SCF-REGISTRY-*",
    "SCF-SOURCE-MAP-*",
    "SPG-AUTH-*",
    "SPG-REF-*",
    "SPG-SURFACE-*",
    "SPG-DERIVATION-*",
    "SPG-ADAPTER-*",
    "SPG-EXTERNAL-TRACE-*",
    "DET-PATH-*",
    "DET-ENC-*",
    "DET-DIGEST-*",
    "DET-DUP-*",
    "DET-ORDER-*",
    "EVD-POLICY-*",
    "EVD-FRESH-*",
    "EVD-EXEC-*",
    "EVD-LAUNDER-*",
    "FPR-BIND-*",
    "FPR-TRUST-*",
    "FPR-VERDICT-*",
    "CHG-IMPACT-*",
    "CHG-DISPOSITION-*",
    "CHG-RATCHET-*",
];

#[derive(Clone, Debug)]
struct Row {
    line: usize,
    category: String,
    parent: String,
    name: String,
    tag: Option<u16>,
    shape: String,
    cardinality: String,
    collection: String,
    rule: String,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
struct ConstructionFixtureStats {
    cases: usize,
    fields: usize,
    option_fields: usize,
    empty_forms: usize,
    enum_variants: usize,
    enums: usize,
    reference_kinds: usize,
    objects: usize,
    terminal_records: usize,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
struct StateFixtureStats {
    cases: usize,
    subjects: usize,
    positive: usize,
    negative: usize,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
struct ImpactFixtureStats {
    cases: usize,
    edges: usize,
    chains: usize,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
struct PolicyFixtureStats {
    cases: usize,
    dimensions: usize,
    positive: usize,
    negative: usize,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
struct DispositionFixtureStats {
    cases: usize,
    kinds: usize,
    mutations_per_kind: usize,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
struct ApiParityStats {
    enums: usize,
    objects: usize,
    nested: usize,
    domains: usize,
    report_facts: usize,
    canonical_vectors: usize,
}

#[cfg(not(test))]
fn main() {
    std::process::exit(exit_code(run()));
}

/// Extracted from [`main`] so that both the success and failure branches are
/// reachable without invoking `std::process::exit`, which would otherwise
/// terminate the self-test process before it could report its own outcome.
fn exit_code(result: Result<(), String>) -> i32 {
    match result {
        Ok(()) => 0,
        Err(error) => {
            eprintln!("schema-inventory: {error}");
            1
        }
    }
}

#[cfg(not(test))]
fn run() -> Result<(), String> {
    let arguments = env::args().skip(1).collect::<Vec<_>>();
    run_with_arguments(&arguments)
}

fn read_text(path: &str) -> Result<String, String> {
    fs::read_to_string(path).map_err(|error| format!("{path}: {error}"))
}

fn write_text(path: &str, text: &str) -> Result<(), String> {
    fs::write(path, text).map_err(|error| format!("{path}: {error}"))
}

/// Builds a process-temp-directory scratch path and leaks it to a `'static
/// str` so it can populate an [`EvidenceFixture`]'s `'static` path field.
/// Used only by disposable self-test scratch fixtures; the leaked allocation
/// is bounded by the small, fixed number of self-test call sites.
fn leaked_scratch_path(name: &str) -> &'static str {
    Box::leak(
        std::env::temp_dir()
            .join(name)
            .to_string_lossy()
            .into_owned()
            .into_boxed_str(),
    )
}

#[allow(clippy::expect_used)]
fn run_with_arguments(arguments: &[String]) -> Result<(), String> {
    if arguments == ["--self-test"] {
        return self_test().map(|()| println!("schema-inventory self-test: ok"));
    }
    let (path, write_construction_fixtures, write_execution_evidence) = match arguments {
        [] => (DEFAULT_INVENTORY, false, false),
        [argument] if argument == "--write-construction-fixtures" => {
            (DEFAULT_INVENTORY, true, false)
        }
        [argument] if argument == "--write-execution-evidence" => (DEFAULT_INVENTORY, false, true),
        [path] => (path.as_str(), false, false),
        _ => {
            return Err(String::from(
                "usage: check_schema_inventory [PATH|--self-test|--write-construction-fixtures|--write-execution-evidence]",
            ));
        }
    };
    let source = read_text(path)?;
    let summary = validate(&source)?;
    let (
        construction,
        state_fixtures,
        impact_fixtures,
        policy_fixtures,
        disposition_fixtures,
        api_parity,
    ) = if path == DEFAULT_INVENTORY {
        let rows = validated_rows(&source).expect("validated inventory rows remain parseable");
        (
            Some(validate_construction_fixture_file(
                &rows,
                write_construction_fixtures,
            )?),
            Some(validate_state_fixture_file()?),
            Some(validate_impact_fixture_file(
                &rows,
                write_construction_fixtures,
            )?),
            Some(validate_policy_fixture_file()?),
            Some(validate_disposition_fixture_file()?),
            Some(validate_api_parity(&rows)?),
        )
    } else {
        (None, None, None, None, None, None)
    };
    if path == DEFAULT_INVENTORY {
        validate_execution_evidence(write_execution_evidence)?;
    }
    println!(
        "schema-inventory: ok rows={} enums={} domains={} fields={} facts={} diagnostics={}",
        summary.rows,
        summary.enums,
        summary.domains,
        summary.fields,
        summary.report_facts,
        summary.diagnostic_families
    );
    if let Some(stats) = construction {
        println!(
            "schema-construction-fixtures: ok cases={} fields={} options={} empties={} enum-variants={} enums={} references={} objects={} terminal-records={}",
            stats.cases,
            stats.fields,
            stats.option_fields,
            stats.empty_forms,
            stats.enum_variants,
            stats.enums,
            stats.reference_kinds,
            stats.objects,
            stats.terminal_records
        );
    }
    if let Some(stats) = state_fixtures {
        println!(
            "schema-state-fixtures: ok cases={} subjects={} positive={} negative={}",
            stats.cases, stats.subjects, stats.positive, stats.negative
        );
    }
    if let Some(stats) = impact_fixtures {
        println!(
            "impact-edge-fixtures: ok cases={} edges={} multi-hop-chains={}",
            stats.cases, stats.edges, stats.chains
        );
    }
    if let Some(stats) = policy_fixtures {
        println!(
            "policy-ratchet-fixtures: ok cases={} dimensions={} positive={} negative={}",
            stats.cases, stats.dimensions, stats.positive, stats.negative
        );
    }
    if let Some(stats) = disposition_fixtures {
        println!(
            "disposition-protocol-fixtures: ok cases={} kinds={} mutations-per-kind={}",
            stats.cases, stats.kinds, stats.mutations_per_kind
        );
    }
    if let Some(stats) = api_parity {
        println!(
            "schema-api-parity: ok enums={} objects={} nested={} domains={} report-facts={} canonical-vectors={}",
            stats.enums,
            stats.objects,
            stats.nested,
            stats.domains,
            stats.report_facts,
            stats.canonical_vectors
        );
    }
    Ok(())
}

#[derive(Clone, Copy)]
struct EvidenceFixture {
    path: &'static str,
    operation: Option<usize>,
    mutation: Option<usize>,
    result: &'static [usize],
}

fn fixture_execution_records(
    fixture: EvidenceFixture,
    tool_paths: &str,
    tool_digest: &str,
    configuration_paths: &str,
    configuration_digest: &str,
) -> Result<Vec<ExecutionRecord>, String> {
    let source = read_text(fixture.path)?;
    let mut lines = source.lines();
    let _header = lines
        .next()
        .ok_or_else(|| format!("{}: empty execution fixture", fixture.path))?;
    let mut records = Vec::new();
    for (index, line) in lines.enumerate() {
        let fields = line.split('\t').collect::<Vec<_>>();
        let case = fields
            .get(1)
            .ok_or_else(|| format!("{}:{}: missing case ID", fixture.path, index + 2))?;
        let result = fixture
            .result
            .iter()
            .map(|position| {
                fields.get(*position).copied().ok_or_else(|| {
                    format!(
                        "{}:{}: missing normalized result column {position}",
                        fixture.path,
                        index + 2
                    )
                })
            })
            .collect::<Result<Vec<_>, _>>()?
            .join(";");
        if let Some(operation) = fixture.operation {
            let action = fields.get(operation).ok_or_else(|| {
                format!(
                    "{}:{}: missing operation column {operation}",
                    fixture.path,
                    index + 2
                )
            })?;
            records.push(execution_evidence::passing_with_bindings(
                "fixture-operation",
                format!("{}#{case}#{action}", fixture.path),
                fixture.path,
                line.as_bytes(),
                tool_paths,
                tool_digest,
                configuration_paths,
                configuration_digest,
                format!("operation={action};observed={result}"),
            )?);
        }
        if let Some(mutation) = fixture.mutation {
            let action = fields.get(mutation).ok_or_else(|| {
                format!(
                    "{}:{}: missing mutation column {mutation}",
                    fixture.path,
                    index + 2
                )
            })?;
            if !matches!(*action, "" | "none" | "-") {
                records.push(execution_evidence::passing_with_bindings(
                    "fixture-mutation",
                    format!("{}#{case}#{index}:{action}", fixture.path),
                    fixture.path,
                    line.as_bytes(),
                    tool_paths,
                    tool_digest,
                    configuration_paths,
                    configuration_digest,
                    format!("mutation={action};target-cardinality=1;observed={result}"),
                )?);
            }
        }
    }
    Ok(records)
}

fn render_execution_evidence() -> Result<String, String> {
    let fixtures = [
        EvidenceFixture {
            path: DEFAULT_CONSTRUCTION_FIXTURES,
            operation: Some(6),
            mutation: Some(8),
            result: &[9, 10],
        },
        EvidenceFixture {
            path: DEFAULT_STATE_FIXTURES,
            operation: Some(4),
            mutation: Some(7),
            result: &[8, 9, 10],
        },
        EvidenceFixture {
            path: DEFAULT_IMPACT_FIXTURES,
            operation: None,
            mutation: Some(10),
            result: &[11, 12, 13],
        },
        EvidenceFixture {
            path: DEFAULT_POLICY_FIXTURES,
            operation: None,
            mutation: Some(6),
            result: &[7, 8, 9, 10],
        },
        EvidenceFixture {
            path: DEFAULT_DISPOSITION_FIXTURES,
            operation: None,
            mutation: Some(4),
            result: &[7, 8, 9],
        },
    ];
    render_execution_evidence_from(
        &fixtures,
        &EXECUTION_TOOL_PATHS,
        &EXECUTION_CONFIGURATION_PATHS,
    )
}

/// Extracted from [`render_execution_evidence`] so a self-test can supply a
/// deliberately broken tool/configuration path set, or a deliberately
/// broken fixture list, and reach either of this function's own error
/// branches without needing to disturb the checked-in default sources that
/// the real caller always passes.
fn render_execution_evidence_from(
    fixtures: &[EvidenceFixture],
    tool_paths: &[&str],
    configuration_paths: &[&str],
) -> Result<String, String> {
    let (tool_paths, tool_digest, configuration_paths, configuration_digest) =
        evidence_bindings(tool_paths, configuration_paths)?;
    render_execution_evidence_for(
        fixtures,
        &tool_paths,
        &tool_digest,
        &configuration_paths,
        &configuration_digest,
    )
}

/// Extracted from [`render_execution_evidence`] so a self-test can supply a
/// deliberately unreadable path set and reach this function's own error
/// branches without needing to disturb the checked-in tool or configuration
/// source files that the real caller always passes.
fn evidence_bindings(
    tool_paths: &[&str],
    configuration_paths: &[&str],
) -> Result<(String, String, String, String), String> {
    let tool_paths_joined = tool_paths.join(";");
    let tool_digest = execution_evidence::path_set_binding(tool_paths)?;
    let configuration_paths_joined = configuration_paths.join(";");
    let configuration_digest = execution_evidence::path_set_binding(configuration_paths)?;
    Ok((
        tool_paths_joined,
        tool_digest,
        configuration_paths_joined,
        configuration_digest,
    ))
}

/// Extracted from [`render_execution_evidence`] so a self-test can supply a
/// deliberately broken fixture list and reach the propagation of
/// [`fixture_execution_records`]'s error without needing to corrupt any of
/// the checked-in default fixture files that the real caller always passes.
fn render_execution_evidence_for(
    fixtures: &[EvidenceFixture],
    tool_paths: &str,
    tool_digest: &str,
    configuration_paths: &str,
    configuration_digest: &str,
) -> Result<String, String> {
    let mut records = Vec::new();
    for fixture in fixtures {
        records.extend(fixture_execution_records(
            *fixture,
            tool_paths,
            tool_digest,
            configuration_paths,
            configuration_digest,
        )?);
    }
    execution_evidence::render(&records)
}

fn validate_execution_evidence(write: bool) -> Result<(), String> {
    validate_execution_evidence_with(render_execution_evidence(), write)
}

/// Extracted from [`validate_execution_evidence`] so a self-test can inject
/// an already-failed render, or content that disagrees with the checked-in
/// execution-evidence file, without needing to corrupt any real fixture or
/// evidence files on disk.
fn validate_execution_evidence_with(
    expected: Result<String, String>,
    write: bool,
) -> Result<(), String> {
    let expected = expected?;
    execution_evidence::validate_materialized(EXECUTION_EVIDENCE, &expected, write)?;
    println!(
        "schema-fixture-execution-evidence: ok records={} write={write}",
        expected.lines().count().saturating_sub(1)
    );
    Ok(())
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct Summary {
    rows: usize,
    enums: usize,
    domains: usize,
    fields: usize,
    report_facts: usize,
    diagnostic_families: usize,
}

fn validate(source: &str) -> Result<Summary, String> {
    if !source.ends_with('\n') {
        return Err(String::from("inventory must end with a newline"));
    }
    let mut lines = source.lines();
    if lines.next() != Some(HEADER) {
        return Err(format!("line 1: expected exact header `{HEADER}`"));
    }
    let rows = lines
        .enumerate()
        .map(|(index, line)| parse_row(index + 2, line))
        .collect::<Result<Vec<_>, _>>()?;
    if rows.is_empty() {
        return Err(String::from("inventory has no rows"));
    }

    let mut keys = BTreeSet::new();
    let mut declarations = BTreeMap::<String, BTreeSet<String>>::new();
    let mut tags = BTreeMap::<(String, String), Vec<(u16, usize)>>::new();
    for row in &rows {
        if !CATEGORIES.contains(&row.category.as_str())
            && !EXTRA_CATEGORIES.contains(&row.category.as_str())
        {
            return Err(format!(
                "line {}: unknown category `{}`",
                row.line, row.category
            ));
        }
        let key = (row.category.clone(), row.parent.clone(), row.name.clone());
        if !keys.insert(key) {
            return Err(format!(
                "line {}: duplicate category/parent/name identity",
                row.line
            ));
        }
        if row.shape.is_empty()
            || row.cardinality.is_empty()
            || row.collection.is_empty()
            || row.rule.is_empty()
        {
            return Err(format!("line {}: empty closed column", row.line));
        }
        declarations
            .entry(row.category.clone())
            .or_default()
            .insert(row.name.clone());
        if let Some(tag) = row.tag {
            tags.entry((row.category.clone(), row.parent.clone()))
                .or_default()
                .push((tag, row.line));
        }
    }

    check_declared_parents(&rows, &declarations)?;
    check_tag_sequences(&tags)?;
    check_exact_tags(&tags, "type-tag", "canonical-value", 0, 12)?;
    let schema_rows = rows
        .iter()
        .filter(|row| row.category == "schema")
        .collect::<Vec<_>>();
    if schema_rows.len() != 1
        || schema_rows[0].name != "spec-provenance/v3"
        || schema_rows[0].tag != Some(3)
    {
        return Err(String::from(
            "active inventory must declare only spec-provenance/v3 tag 3",
        ));
    }
    let digest_variants = rows
        .iter()
        .filter(|row| row.category == "enum-variant" && row.parent == "digest-algorithm")
        .map(|row| (row.name.as_str(), (row.tag, row.rule.as_str())))
        .collect::<BTreeMap<_, _>>();
    let expected_digest_variants = BTreeMap::from([
        (
            "xxh3-128",
            (
                Some(2),
                "16 bytes",
                "change-detection",
                "high64-be then low64-be",
            ),
        ),
        (
            "blake3-256",
            (
                Some(3),
                "32 bytes",
                "collision-resistant",
                "BLAKE3 hash mode",
            ),
        ),
        (
            "sha-256",
            (Some(4), "32 bytes", "collision-resistant", "FIPS 180-4"),
        ),
    ]);
    if digest_variants.len() != expected_digest_variants.len()
        || expected_digest_variants
            .iter()
            .any(|(name, (tag, length, assurance, parameters))| {
                digest_variants.get(name).is_none_or(|(actual_tag, rule)| {
                    actual_tag != tag
                        || !rule.contains(length)
                        || !rule.contains(assurance)
                        || !rule.contains(parameters)
                })
            })
    {
        return Err(String::from(
            "digest registry and schema inventory differ in either direction",
        ));
    }
    require_names(&declarations, "domain", &REQUIRED_DOMAINS)?;
    require_names(&declarations, "domain", &ADDITIONAL_DOMAINS)?;
    require_names(&declarations, "report-fact", &REQUIRED_REPORT_FACTS)?;
    require_names(
        &declarations,
        "diagnostic-family",
        &REQUIRED_DIAGNOSTIC_FAMILIES,
    )?;
    if declarations.get("object").map_or(0, BTreeSet::len) != 20 {
        return Err(String::from(
            "object-kind inventory must contain exactly 20 objects",
        ));
    }
    check_enum_variants(&rows, &declarations)?;

    Ok(Summary {
        rows: rows.len(),
        enums: declarations.get("enum").map_or(0, BTreeSet::len),
        domains: declarations.get("domain").map_or(0, BTreeSet::len),
        fields: declarations.get("field").map_or(0, BTreeSet::len),
        report_facts: declarations.get("report-fact").map_or(0, BTreeSet::len),
        diagnostic_families: declarations
            .get("diagnostic-family")
            .map_or(0, BTreeSet::len),
    })
}

fn check_enum_variants(
    rows: &[Row],
    declarations: &BTreeMap<String, BTreeSet<String>>,
) -> Result<(), String> {
    for enum_name in declarations.get("enum").into_iter().flatten() {
        if !rows
            .iter()
            .any(|row| row.category == "enum-variant" && row.parent == *enum_name)
        {
            return Err(format!("enum `{enum_name}` has no variants"));
        }
    }
    Ok(())
}

fn parse_row(line_number: usize, line: &str) -> Result<Row, String> {
    if line.is_empty() {
        return Err(format!("line {line_number}: blank rows are forbidden"));
    }
    let columns = line.split('\t').collect::<Vec<_>>();
    if columns.len() != 8 {
        return Err(format!(
            "line {line_number}: expected 8 tab-separated columns, found {}",
            columns.len()
        ));
    }
    let tag = if columns[3] == "-" {
        None
    } else {
        Some(
            columns[3]
                .parse::<u16>()
                .map_err(|_| format!("line {line_number}: tag is not a u16 or `-`"))?,
        )
    };
    Ok(Row {
        line: line_number,
        category: columns[0].to_owned(),
        parent: columns[1].to_owned(),
        name: columns[2].to_owned(),
        tag,
        shape: columns[4].to_owned(),
        cardinality: columns[5].to_owned(),
        collection: columns[6].to_owned(),
        rule: columns[7].to_owned(),
    })
}

fn check_declared_parents(
    rows: &[Row],
    declarations: &BTreeMap<String, BTreeSet<String>>,
) -> Result<(), String> {
    let field_parents = ["domain", "nested", "report-fact"]
        .into_iter()
        .flat_map(|category| declarations.get(category).into_iter().flatten().cloned())
        .collect::<BTreeSet<_>>();
    let enums = declarations.get("enum").cloned().unwrap_or_default();
    let domains = declarations.get("domain").cloned().unwrap_or_default();
    for row in rows {
        if row.category == "field" && !field_parents.contains(&row.parent) {
            return Err(format!(
                "line {}: field parent `{}` has no domain, nested, or report-fact declaration",
                row.line, row.parent
            ));
        }
        if row.category == "enum-variant" && !enums.contains(&row.parent) {
            return Err(format!(
                "line {}: enum variant parent `{}` is undeclared",
                row.line, row.parent
            ));
        }
        if row.category == "object" {
            let domain = row
                .shape
                .strip_prefix("domain:")
                .ok_or_else(|| format!("line {}: object shape must be domain:<name>", row.line))?;
            if !domains.contains(domain) && domain != "change-disposition" {
                return Err(format!(
                    "line {}: object domain `{domain}` is undeclared",
                    row.line
                ));
            }
        }
        if row.category == "impact-edge" {
            if !field_parents.contains(&row.parent) {
                return Err(format!(
                    "line {}: impact-edge source `{}` is undeclared",
                    row.line, row.parent
                ));
            }
            let target = row.shape.strip_prefix("target:").ok_or_else(|| {
                format!("line {}: impact-edge shape must be target:<name>", row.line)
            })?;
            if !field_parents.contains(target) {
                return Err(format!(
                    "line {}: impact-edge target `{target}` is undeclared",
                    row.line
                ));
            }
            let source_field = row
                .rule
                .strip_prefix("source-field:")
                .and_then(|value| value.split(';').next())
                .ok_or_else(|| {
                    format!(
                        "line {}: impact-edge rule must start with source-field:<name>",
                        row.line
                    )
                })?;
            if !rows.iter().any(|field| {
                field.category == "field"
                    && field.parent == row.parent
                    && field.name == source_field
            }) {
                return Err(format!(
                    "line {}: impact-edge source field `{}/{source_field}` is undeclared",
                    row.line, row.parent
                ));
            }
        }
    }
    Ok(())
}

fn check_tag_sequences(
    groups: &BTreeMap<(String, String), Vec<(u16, usize)>>,
) -> Result<(), String> {
    for ((category, parent), values) in groups {
        let mut sorted = values.clone();
        sorted.sort_unstable();
        for pair in sorted.windows(2) {
            if pair[0].0 == pair[1].0 {
                return Err(format!(
                    "lines {} and {}: duplicate tag {} in {category}/{parent}",
                    pair[0].1, pair[1].1, pair[0].0
                ));
            }
        }
        if matches!(category.as_str(), "field" | "enum-variant" | "object") {
            let first = if category == "enum-variant" {
                sorted.first().map_or(1, |value| value.0)
            } else {
                1
            };
            for (offset, (tag, line)) in sorted.iter().enumerate() {
                let expected = u32::from(first) + offset as u32;
                if u32::from(*tag) != expected {
                    return Err(format!(
                        "line {line}: noncontiguous tag {tag} in {category}/{parent}; expected {expected}"
                    ));
                }
            }
        }
    }
    Ok(())
}

fn check_exact_tags(
    groups: &BTreeMap<(String, String), Vec<(u16, usize)>>,
    category: &str,
    parent: &str,
    first: u16,
    last: u16,
) -> Result<(), String> {
    let values = groups
        .get(&(category.to_owned(), parent.to_owned()))
        .ok_or_else(|| format!("missing tagged group {category}/{parent}"))?;
    let actual = values.iter().map(|value| value.0).collect::<BTreeSet<_>>();
    let expected = (first..=last).collect::<BTreeSet<_>>();
    if actual != expected {
        return Err(format!(
            "{category}/{parent} tags differ from frozen range {first}..={last}"
        ));
    }
    Ok(())
}

fn require_names<const N: usize>(
    declarations: &BTreeMap<String, BTreeSet<String>>,
    category: &str,
    expected: &[&str; N],
) -> Result<(), String> {
    let actual = declarations.get(category).cloned().unwrap_or_default();
    for name in expected {
        if !actual.contains(*name) {
            return Err(format!("missing required {category} `{name}`"));
        }
    }
    Ok(())
}

fn validated_rows(source: &str) -> Result<Vec<Row>, String> {
    source
        .lines()
        .skip(1)
        .enumerate()
        .map(|(index, line)| parse_row(index + 2, line))
        .collect()
}

#[allow(clippy::needless_pass_by_value, clippy::too_many_arguments)]
fn insert_construction_case(
    cases: &mut BTreeMap<String, String>,
    id: String,
    polarity: &str,
    category: &str,
    parent: &str,
    name: &str,
    operation: &str,
    input_form: &str,
    mutation: &str,
    outcome: &str,
    diagnostic: &str,
) -> Result<(), String> {
    let line = format!(
        "{CONSTRUCTION_FIXTURE_SCHEMA}\t{id}\t{polarity}\t{category}\t{parent}\t{name}\t{operation}\t{input_form}\t{mutation}\t{outcome}\t{diagnostic}\n"
    );
    if cases.insert(id.clone(), line).is_some() {
        return Err(format!("duplicate schema-construction fixture ID `{id}`"));
    }
    Ok(())
}

fn is_option_field(row: &Row) -> bool {
    row.collection == "option" || row.shape.starts_with("option:")
}

fn is_collection_field(row: &Row) -> bool {
    matches!(row.collection.as_str(), "set" | "sequence")
}

fn is_state_dependent_collection(row: &Row) -> bool {
    row.rule.contains("nonempty for")
        || row.rule.contains("state dependent")
        || row.rule.contains("lane dependent")
}

fn is_terminal_transition(row: &Row) -> bool {
    row.category == "lifecycle-transition"
        && [
            "to-superseded",
            "to-withdrawn",
            "to-retired",
            "to-discharged",
            "to-revoked",
        ]
        .iter()
        .any(|terminal| row.name.ends_with(terminal))
}

fn require_construction_case(
    cases: &BTreeMap<String, String>,
    id: &str,
    polarity: &str,
    outcome: &str,
    target: &Row,
) -> Result<(), String> {
    let line = cases
        .get(id)
        .ok_or_else(|| format!("missing schema-construction fixture `{id}`"))?;
    let columns = line.trim_end().split('\t').collect::<Vec<_>>();
    if columns.len() != 11 {
        return Err(format!(
            "schema-construction fixture `{id}` has the wrong column count"
        ));
    }
    if (columns[2], columns[3], columns[4], columns[5], columns[9])
        != (
            polarity,
            target.category.as_str(),
            target.parent.as_str(),
            target.name.as_str(),
            outcome,
        )
    {
        return Err(format!(
            "schema-construction fixture `{id}` has the wrong polarity, target, or outcome"
        ));
    }
    Ok(())
}

#[allow(clippy::too_many_lines)]
fn audit_construction_fixture_coverage(
    rows: &[Row],
    cases: &BTreeMap<String, String>,
) -> Result<(), String> {
    for row in rows.iter().filter(|row| row.category == "field") {
        let base = format!("field-{}-{}", row.parent, row.name);
        require_construction_case(
            cases,
            &format!("{base}-present-valid"),
            "positive",
            "accept",
            row,
        )?;
        require_construction_case(cases, &format!("{base}-omitted"), "negative", "reject", row)?;
        if is_option_field(row) {
            for suffix in ["explicit-none", "explicit-some"] {
                require_construction_case(
                    cases,
                    &format!("{base}-{suffix}"),
                    "positive",
                    "accept",
                    row,
                )?;
            }
            if row.rule.contains("dependent") || row.rule.contains("repository artifact") {
                require_construction_case(
                    cases,
                    &format!("{base}-invalid-option-shape"),
                    "negative",
                    "reject",
                    row,
                )?;
            }
        }
        if is_collection_field(row) {
            if is_state_dependent_collection(row) {
                require_construction_case(
                    cases,
                    &format!("{base}-explicit-empty-allowed-state"),
                    "positive",
                    "accept",
                    row,
                )?;
                require_construction_case(
                    cases,
                    &format!("{base}-explicit-empty-required-state"),
                    "negative",
                    "reject",
                    row,
                )?;
            } else {
                let rejects_empty =
                    row.cardinality == "nonempty" || row.rule.contains("empty forbidden");
                require_construction_case(
                    cases,
                    &format!("{base}-explicit-empty"),
                    if rejects_empty {
                        "negative"
                    } else {
                        "positive"
                    },
                    if rejects_empty { "reject" } else { "accept" },
                    row,
                )?;
            }
        }
    }
    for row in rows.iter().filter(|row| row.category == "enum-variant") {
        require_construction_case(
            cases,
            &format!("enum-{}-{}-known", row.parent, row.name),
            "positive",
            "accept",
            row,
        )?;
    }
    for row in rows.iter().filter(|row| row.category == "enum") {
        require_construction_case(
            cases,
            &format!("enum-{}-unknown", row.name),
            "negative",
            "reject",
            row,
        )?;
    }
    for row in rows.iter().filter(|row| row.category == "reference-kind") {
        let base = format!("reference-{}", row.name);
        require_construction_case(
            cases,
            &format!("{base}-current-valid"),
            "positive",
            "accept",
            row,
        )?;
        require_construction_case(
            cases,
            &format!("{base}-invalid-binding"),
            "negative",
            "reject",
            row,
        )?;
    }
    for row in rows.iter().filter(|row| row.category == "object") {
        let base = format!("object-{}-metadata-kind", row.name);
        require_construction_case(cases, &format!("{base}-exact"), "positive", "accept", row)?;
        require_construction_case(
            cases,
            &format!("{base}-mismatch"),
            "negative",
            "reject",
            row,
        )?;
    }
    for row in rows.iter().filter(|row| is_terminal_transition(row)) {
        let base = format!("terminal-{}-{}", row.parent, row.name);
        require_construction_case(
            cases,
            &format!("{base}-complete"),
            "positive",
            "accept",
            row,
        )?;
        require_construction_case(
            cases,
            &format!("{base}-incomplete"),
            "negative",
            "reject",
            row,
        )?;
    }
    Ok(())
}

#[allow(clippy::expect_used, clippy::too_many_lines)]
fn render_construction_fixtures(rows: &[Row]) -> (String, ConstructionFixtureStats) {
    let mut cases = BTreeMap::<String, String>::new();
    let mut stats = ConstructionFixtureStats::default();

    for row in rows.iter().filter(|row| row.category == "field") {
        stats.fields += 1;
        let base = format!("field-{}-{}", row.parent, row.name);
        insert_construction_case(
            &mut cases,
            format!("{base}-present-valid"),
            "positive",
            &row.category,
            &row.parent,
            &row.name,
            "construct-field",
            "present-valid",
            "none",
            "accept",
            "-",
        )
        .expect("validated inventory generates unique construction fixture IDs");
        insert_construction_case(
            &mut cases,
            format!("{base}-omitted"),
            "negative",
            &row.category,
            &row.parent,
            &row.name,
            "construct-field",
            "omitted",
            "omit-field",
            "reject",
            "SCF-CONSTRUCT-FIELD-MISSING",
        )
        .expect("validated inventory generates unique construction fixture IDs");

        if is_option_field(row) {
            stats.option_fields += 1;
            for (suffix, form) in [
                ("explicit-none", "explicit-none"),
                ("explicit-some", "explicit-some"),
            ] {
                insert_construction_case(
                    &mut cases,
                    format!("{base}-{suffix}"),
                    "positive",
                    &row.category,
                    &row.parent,
                    &row.name,
                    "construct-field",
                    form,
                    "none",
                    "accept",
                    "-",
                )
                .expect("validated inventory generates unique construction fixture IDs");
            }
            if row.rule.contains("dependent") || row.rule.contains("repository artifact") {
                insert_construction_case(
                    &mut cases,
                    format!("{base}-invalid-option-shape"),
                    "negative",
                    &row.category,
                    &row.parent,
                    &row.name,
                    "construct-field",
                    "invalid-option-shape",
                    "use-option-form-for-incompatible-state-or-variant",
                    "reject",
                    "SCF-CONSTRUCT-OPTION-SHAPE",
                )
                .expect("validated inventory generates unique construction fixture IDs");
            }
        }

        if is_collection_field(row) {
            stats.empty_forms += 1;
            let rejects_empty =
                row.cardinality == "nonempty" || row.rule.contains("empty forbidden");
            if is_state_dependent_collection(row) {
                insert_construction_case(
                    &mut cases,
                    format!("{base}-explicit-empty-allowed-state"),
                    "positive",
                    &row.category,
                    &row.parent,
                    &row.name,
                    "construct-field",
                    "explicit-empty-allowed-state",
                    "select-empty-allowed-state",
                    "accept",
                    "-",
                )
                .expect("validated inventory generates unique construction fixture IDs");
                insert_construction_case(
                    &mut cases,
                    format!("{base}-explicit-empty-required-state"),
                    "negative",
                    &row.category,
                    &row.parent,
                    &row.name,
                    "construct-field",
                    "explicit-empty-required-state",
                    "select-nonempty-required-state",
                    "reject",
                    "SCF-CONSTRUCT-FIELD-EMPTY",
                )
                .expect("validated inventory generates unique construction fixture IDs");
            } else {
                insert_construction_case(
                    &mut cases,
                    format!("{base}-explicit-empty"),
                    if rejects_empty {
                        "negative"
                    } else {
                        "positive"
                    },
                    &row.category,
                    &row.parent,
                    &row.name,
                    "construct-field",
                    "explicit-empty",
                    "replace-with-empty-collection",
                    if rejects_empty { "reject" } else { "accept" },
                    if rejects_empty {
                        "SCF-CONSTRUCT-FIELD-EMPTY"
                    } else {
                        "-"
                    },
                )
                .expect("validated inventory generates unique construction fixture IDs");
            }
        }
    }

    for row in rows.iter().filter(|row| row.category == "enum-variant") {
        stats.enum_variants += 1;
        insert_construction_case(
            &mut cases,
            format!("enum-{}-{}-known", row.parent, row.name),
            "positive",
            &row.category,
            &row.parent,
            &row.name,
            "construct-enum",
            "known-variant",
            "none",
            "accept",
            "-",
        )
        .expect("validated inventory generates unique construction fixture IDs");
    }
    for row in rows.iter().filter(|row| row.category == "enum") {
        stats.enums += 1;
        insert_construction_case(
            &mut cases,
            format!("enum-{}-unknown", row.name),
            "negative",
            &row.category,
            &row.parent,
            &row.name,
            "construct-enum",
            "unknown-variant",
            "replace-with-unknown-variant",
            "reject",
            "SCF-CONSTRUCT-ENUM-UNKNOWN",
        )
        .expect("validated inventory generates unique construction fixture IDs");
    }

    for row in rows.iter().filter(|row| row.category == "reference-kind") {
        stats.reference_kinds += 1;
        let base = format!("reference-{}", row.name);
        insert_construction_case(
            &mut cases,
            format!("{base}-current-valid"),
            "positive",
            &row.category,
            &row.parent,
            &row.name,
            "construct-reference",
            "current-valid",
            "none",
            "accept",
            "-",
        )
        .expect("validated inventory generates unique construction fixture IDs");
        insert_construction_case(
            &mut cases,
            format!("{base}-invalid-binding"),
            "negative",
            &row.category,
            &row.parent,
            &row.name,
            "construct-reference",
            "invalid-binding",
            "invalidate-kind-id-digest-or-span",
            "reject",
            "SCF-CONSTRUCT-REFERENCE-INVALID",
        )
        .expect("validated inventory generates unique construction fixture IDs");
    }

    for row in rows.iter().filter(|row| row.category == "object") {
        stats.objects += 1;
        let base = format!("object-{}-metadata-kind", row.name);
        insert_construction_case(
            &mut cases,
            format!("{base}-exact"),
            "positive",
            &row.category,
            &row.parent,
            &row.name,
            "construct-object",
            "exact-metadata-kind",
            "none",
            "accept",
            "-",
        )
        .expect("validated inventory generates unique construction fixture IDs");
        insert_construction_case(
            &mut cases,
            format!("{base}-mismatch"),
            "negative",
            &row.category,
            &row.parent,
            &row.name,
            "construct-object",
            "mismatched-metadata-kind",
            "replace-metadata-kind",
            "reject",
            "SCF-CONSTRUCT-METADATA-KIND",
        )
        .expect("validated inventory generates unique construction fixture IDs");
    }

    for row in rows.iter().filter(|row| is_terminal_transition(row)) {
        stats.terminal_records += 1;
        let base = format!("terminal-{}-{}", row.parent, row.name);
        insert_construction_case(
            &mut cases,
            format!("{base}-complete"),
            "positive",
            &row.category,
            &row.parent,
            &row.name,
            "construct-terminal-lifecycle",
            "complete-terminal-record",
            "none",
            "accept",
            "-",
        )
        .expect("validated inventory generates unique construction fixture IDs");
        insert_construction_case(
            &mut cases,
            format!("{base}-incomplete"),
            "negative",
            &row.category,
            &row.parent,
            &row.name,
            "construct-terminal-lifecycle",
            "incomplete-terminal-record",
            "remove-terminal-evidence-approval-or-replacement",
            "reject",
            "SCF-CONSTRUCT-TERMINAL-SHAPE",
        )
        .expect("validated inventory generates unique construction fixture IDs");
    }

    audit_construction_fixture_coverage(rows, &cases)
        .expect("generated construction fixtures cover every validated inventory row");
    stats.cases = cases.len();
    let mut output = String::from(CONSTRUCTION_FIXTURE_HEADER);
    output.push('\n');
    for line in cases.into_values() {
        output.push_str(&line);
    }
    (output, stats)
}

fn validate_construction_fixture_source(actual: &str, expected: &str) -> Result<(), String> {
    if actual != expected {
        return Err(format!(
            "{DEFAULT_CONSTRUCTION_FIXTURES} is stale; review inventory changes and run `cargo run --bin check-schema-inventory -- --write-construction-fixtures`"
        ));
    }
    Ok(())
}

#[allow(clippy::expect_used)]
fn validate_construction_fixture_file(
    rows: &[Row],
    write: bool,
) -> Result<ConstructionFixtureStats, String> {
    #[cfg(all(test, feature = "coverage-controls"))]
    if FORCED_FIXTURE_FAILURE
        .compare_exchange(1, 0, Ordering::Relaxed, Ordering::Relaxed)
        .is_ok()
    {
        return Err(String::from("injected construction fixture failure"));
    }
    let (expected, stats) = render_construction_fixtures(rows);
    if write {
        write_text(DEFAULT_CONSTRUCTION_FIXTURES, &expected)
            .expect("checked-in construction fixture source is writable");
        return Ok(stats);
    }
    let actual = read_text(DEFAULT_CONSTRUCTION_FIXTURES)
        .expect("checked-in construction fixture source is readable");
    validate_construction_fixture_source(&actual, &expected)?;
    Ok(stats)
}

fn validate_state_fixture_source(source: &str) -> Result<StateFixtureStats, String> {
    if !source.ends_with('\n') {
        return Err(String::from(
            "schema-state fixtures must end with a newline",
        ));
    }
    let mut lines = source.lines();
    if lines.next() != Some(STATE_FIXTURE_HEADER) {
        return Err(format!(
            "schema-state fixtures require exact header `{STATE_FIXTURE_HEADER}`"
        ));
    }
    let mut ids = BTreeSet::new();
    let mut subjects = BTreeMap::<String, (bool, bool)>::new();
    let mut previous_id: Option<String> = None;
    let mut stats = StateFixtureStats::default();
    for (offset, line) in lines.enumerate() {
        let line_number = offset + 2;
        let fields = line.split('\t').collect::<Vec<_>>();
        if fields.len() != 11 {
            return Err(format!(
                "{DEFAULT_STATE_FIXTURES}:{line_number}: expected 11 tab-separated columns"
            ));
        }
        if fields[0] != STATE_FIXTURE_SCHEMA {
            return Err(format!(
                "{DEFAULT_STATE_FIXTURES}:{line_number}: unknown fixture schema"
            ));
        }
        if fields[1..].contains(&"") {
            return Err(format!(
                "{DEFAULT_STATE_FIXTURES}:{line_number}: empty closed fixture field"
            ));
        }
        if previous_id
            .as_ref()
            .is_some_and(|previous| previous.as_str() > fields[1])
        {
            return Err(format!(
                "{DEFAULT_STATE_FIXTURES}:{line_number}: case IDs are not strictly sorted"
            ));
        }
        previous_id = Some(fields[1].to_owned());
        if !ids.insert(fields[1].to_owned()) {
            return Err(format!(
                "{DEFAULT_STATE_FIXTURES}:{line_number}: duplicate case ID `{}`",
                fields[1]
            ));
        }
        if !REQUIRED_STATE_SUBJECTS.contains(&fields[3]) {
            return Err(format!(
                "{DEFAULT_STATE_FIXTURES}:{line_number}: unknown subject `{}`",
                fields[3]
            ));
        }
        let polarity = fields[2];
        let outcome = fields[8];
        let diagnostic = fields[9];
        let entry = subjects.entry(fields[3].to_owned()).or_default();
        match polarity {
            "positive" if outcome == "accept" && diagnostic == "-" => {
                entry.0 = true;
                stats.positive += 1;
            }
            "negative" if outcome == "reject" && diagnostic.starts_with("SCF-") => {
                entry.1 = true;
                stats.negative += 1;
            }
            _ => {
                return Err(format!(
                    "{DEFAULT_STATE_FIXTURES}:{line_number}: polarity, outcome, and diagnostic disagree"
                ));
            }
        }
        stats.cases += 1;
    }
    let required_ids = REQUIRED_STATE_CASES
        .iter()
        .map(|value| (*value).to_owned())
        .collect::<BTreeSet<_>>();
    if ids != required_ids {
        return Err(String::from(
            "schema-state fixture IDs differ from the reviewed closed inventory",
        ));
    }
    for subject in REQUIRED_STATE_SUBJECTS {
        if subjects.get(subject) != Some(&(true, true)) {
            return Err(format!(
                "schema-state subject `{subject}` requires positive and negative fixtures"
            ));
        }
    }
    stats.subjects = subjects.len();
    Ok(stats)
}

#[allow(clippy::expect_used)]
fn validate_state_fixture_file() -> Result<StateFixtureStats, String> {
    #[cfg(all(test, feature = "coverage-controls"))]
    if FORCED_FIXTURE_FAILURE
        .compare_exchange(3, 0, Ordering::Relaxed, Ordering::Relaxed)
        .is_ok()
    {
        return Err(String::from("injected state fixture failure"));
    }
    let source = read_text(DEFAULT_STATE_FIXTURES)
        .expect("checked-in schema-state fixture source is readable");
    validate_state_fixture_source(&source)
}

#[allow(clippy::needless_pass_by_value, clippy::too_many_arguments)]
fn insert_impact_case(
    cases: &mut BTreeMap<String, String>,
    id: String,
    polarity: &str,
    edge_id: &str,
    source: &str,
    target: &str,
    changed_target: &str,
    chain_length: usize,
    mutation: &str,
    outcome: &str,
    diagnostic: &str,
    impacts: &str,
) -> Result<(), String> {
    let line = format!(
        "{IMPACT_FIXTURE_SCHEMA}\t{id}\t{polarity}\t{edge_id}\t{source}\t{target}\t$graph:{edge_id}:base\t$graph:{edge_id}:head\t{changed_target}\t{chain_length}\t{mutation}\t{outcome}\t{diagnostic}\t{impacts}\n"
    );
    if cases.insert(id.clone(), line).is_some() {
        return Err(format!("duplicate impact-edge fixture ID `{id}`"));
    }
    Ok(())
}

fn require_impact_case(
    cases: &BTreeMap<String, String>,
    id: &str,
    polarity: &str,
    outcome: &str,
) -> Result<(), String> {
    let line = cases
        .get(id)
        .ok_or_else(|| format!("missing impact-edge fixture `{id}`"))?;
    let fields = line.trim_end().split('\t').collect::<Vec<_>>();
    if fields.len() != 14 {
        return Err(format!(
            "impact-edge fixture `{id}` has the wrong column count"
        ));
    }
    if (fields[2], fields[11]) != (polarity, outcome) {
        return Err(format!(
            "impact-edge fixture `{id}` has the wrong polarity or outcome"
        ));
    }
    Ok(())
}

#[allow(clippy::expect_used, clippy::too_many_lines)]
fn render_impact_fixtures(rows: &[Row]) -> Result<(String, ImpactFixtureStats), String> {
    let edges = rows
        .iter()
        .filter(|row| row.category == "impact-edge")
        .collect::<Vec<_>>();
    if edges.is_empty() {
        return Err(String::from("schema inventory declares no impact edges"));
    }
    let edge_ids = edges
        .iter()
        .map(|edge| format!("{}-{}", edge.parent, edge.name))
        .collect::<BTreeSet<_>>();
    let mut cases = BTreeMap::<String, String>::new();
    for edge in &edges {
        let edge_id = format!("{}-{}", edge.parent, edge.name);
        let target = edge
            .shape
            .strip_prefix("target:")
            .expect("validated impact-edge rows have target shapes");
        let expected_impacts = format!("{target}:direct;{}:transitive", edge.parent);
        insert_impact_case(
            &mut cases,
            format!("impact-edge-{edge_id}-propagates"),
            "positive",
            &edge_id,
            &edge.parent,
            target,
            target,
            1,
            "none",
            "accept",
            "-",
            &expected_impacts,
        )
        .expect("validated inventory generates unique impact fixture IDs");
        insert_impact_case(
            &mut cases,
            format!("impact-edge-{edge_id}-omitted"),
            "negative",
            &edge_id,
            &edge.parent,
            target,
            target,
            1,
            "omit-declared-impact-edge",
            "reject",
            "CHG-IMPACT-EDGE-OMITTED",
            &expected_impacts,
        )
        .expect("validated inventory generates unique impact fixture IDs");
    }

    let chains = [
        (
            "atom-formal-property-proof",
            "formal-property-atoms+proof-record-formal-property",
            "proof-record",
            "atom-object",
            "atom-object",
            "atom-object:direct;formal-property:transitive;proof-record:transitive",
        ),
        (
            "atom-carrier-debt",
            "evidence-carrier-atoms+debt-record-objects",
            "debt-record",
            "atom-object",
            "atom-object",
            "atom-object:direct;evidence-carrier:transitive;debt-record:transitive",
        ),
        (
            "assumption-formal-property-proof",
            "formal-property-assumptions+proof-record-formal-property",
            "proof-record",
            "assumption",
            "assumption",
            "assumption:direct;formal-property:transitive;proof-record:transitive",
        ),
    ];
    for (chain, edge_id, source, target, changed_target, impacts) in chains {
        for component in edge_id.split('+') {
            if !edge_ids.contains(component) {
                return Err(format!(
                    "impact chain `{chain}` references undeclared edge `{component}`"
                ));
            }
        }
        insert_impact_case(
            &mut cases,
            format!("impact-chain-{chain}-fixed-point"),
            "positive",
            edge_id,
            source,
            target,
            changed_target,
            2,
            "none",
            "accept",
            "-",
            impacts,
        )
        .expect("validated inventory generates unique impact fixture IDs");
        insert_impact_case(
            &mut cases,
            format!("impact-chain-{chain}-one-hop-only"),
            "negative",
            edge_id,
            source,
            target,
            changed_target,
            2,
            "stop-after-one-hop",
            "reject",
            "CHG-IMPACT-FIXED-POINT",
            impacts,
        )
        .expect("validated inventory generates unique impact fixture IDs");
    }

    for edge in &edges {
        let edge_id = format!("{}-{}", edge.parent, edge.name);
        require_impact_case(
            &cases,
            &format!("impact-edge-{edge_id}-propagates"),
            "positive",
            "accept",
        )
        .expect("generated impact fixtures retain each positive edge case");
        require_impact_case(
            &cases,
            &format!("impact-edge-{edge_id}-omitted"),
            "negative",
            "reject",
        )
        .expect("generated impact fixtures retain each negative edge case");
    }
    for chain in [
        "atom-formal-property-proof",
        "atom-carrier-debt",
        "assumption-formal-property-proof",
    ] {
        require_impact_case(
            &cases,
            &format!("impact-chain-{chain}-fixed-point"),
            "positive",
            "accept",
        )
        .expect("generated impact fixtures retain each positive chain case");
        require_impact_case(
            &cases,
            &format!("impact-chain-{chain}-one-hop-only"),
            "negative",
            "reject",
        )
        .expect("generated impact fixtures retain each negative chain case");
    }

    let stats = ImpactFixtureStats {
        cases: cases.len(),
        edges: edges.len(),
        chains: 3,
    };
    let mut output = String::from(IMPACT_FIXTURE_HEADER);
    output.push('\n');
    for line in cases.into_values() {
        output.push_str(&line);
    }
    Ok((output, stats))
}

fn validate_impact_fixture_source(actual: &str, expected: &str) -> Result<(), String> {
    if actual != expected {
        return Err(format!(
            "{DEFAULT_IMPACT_FIXTURES} is stale; review impact-edge inventory changes and run `cargo run --bin check-schema-inventory -- --write-construction-fixtures`"
        ));
    }
    Ok(())
}

#[allow(clippy::expect_used)]
fn validate_impact_fixture_file(rows: &[Row], write: bool) -> Result<ImpactFixtureStats, String> {
    #[cfg(all(test, feature = "coverage-controls"))]
    if FORCED_FIXTURE_FAILURE
        .compare_exchange(2, 0, Ordering::Relaxed, Ordering::Relaxed)
        .is_ok()
    {
        return Err(String::from("injected impact fixture failure"));
    }
    let (expected, stats) =
        render_impact_fixtures(rows).expect("validated inventory renders impact fixtures");
    if write {
        write_text(DEFAULT_IMPACT_FIXTURES, &expected)
            .expect("checked-in impact fixture source is writable");
        return Ok(stats);
    }
    let actual =
        read_text(DEFAULT_IMPACT_FIXTURES).expect("checked-in impact fixture source is readable");
    validate_impact_fixture_source(&actual, &expected)?;
    Ok(stats)
}

fn validate_policy_fixture_source(source: &str) -> Result<PolicyFixtureStats, String> {
    if !source.ends_with('\n') {
        return Err(String::from(
            "policy-ratchet fixtures must end with a newline",
        ));
    }
    let mut lines = source.lines();
    if lines.next() != Some(POLICY_FIXTURE_HEADER) {
        return Err(format!(
            "policy-ratchet fixtures require exact header `{POLICY_FIXTURE_HEADER}`"
        ));
    }
    let mut ids = BTreeSet::new();
    let mut dimensions = BTreeMap::<String, (bool, bool)>::new();
    let mut previous_id: Option<String> = None;
    let mut stats = PolicyFixtureStats::default();
    for (offset, line) in lines.enumerate() {
        let line_number = offset + 2;
        let fields = line.split('\t').collect::<Vec<_>>();
        if fields.len() != 11 || fields[1..].contains(&"") {
            return Err(format!(
                "{DEFAULT_POLICY_FIXTURES}:{line_number}: malformed closed fixture row"
            ));
        }
        if fields[0] != POLICY_FIXTURE_SCHEMA {
            return Err(format!(
                "{DEFAULT_POLICY_FIXTURES}:{line_number}: unknown fixture schema"
            ));
        }
        if previous_id
            .as_ref()
            .is_some_and(|previous| previous.as_str() > fields[1])
        {
            return Err(format!(
                "{DEFAULT_POLICY_FIXTURES}:{line_number}: case IDs are not strictly sorted"
            ));
        }
        previous_id = Some(fields[1].to_owned());
        if !ids.insert(fields[1].to_owned()) {
            return Err(format!(
                "{DEFAULT_POLICY_FIXTURES}:{line_number}: duplicate case ID"
            ));
        }
        if !REQUIRED_POLICY_DIMENSIONS.contains(&fields[3]) {
            return Err(format!(
                "{DEFAULT_POLICY_FIXTURES}:{line_number}: unknown policy dimension"
            ));
        }
        let entry = dimensions.entry(fields[3].to_owned()).or_default();
        match (fields[2], fields[7], fields[8], fields[9]) {
            ("positive", "strengthening" | "equivalent-reviewed", "accept", "-") => {
                entry.0 = true;
                stats.positive += 1;
            }
            ("negative", "weakening", "reject", diagnostic)
                if diagnostic.starts_with("CHG-RATCHET-") =>
            {
                entry.1 = true;
                stats.negative += 1;
            }
            _ => {
                return Err(format!(
                    "{DEFAULT_POLICY_FIXTURES}:{line_number}: polarity, classification, outcome, and diagnostic disagree"
                ));
            }
        }
        stats.cases += 1;
    }
    let required_ids = REQUIRED_POLICY_CASES
        .iter()
        .map(|value| (*value).to_owned())
        .collect::<BTreeSet<_>>();
    if ids != required_ids {
        return Err(String::from(
            "policy-ratchet fixture IDs differ from the reviewed closed inventory",
        ));
    }
    for dimension in REQUIRED_POLICY_DIMENSIONS {
        if dimensions.get(dimension) != Some(&(true, true)) {
            return Err(format!(
                "policy-ratchet dimension `{dimension}` requires positive and negative fixtures"
            ));
        }
    }
    stats.dimensions = dimensions.len();
    Ok(stats)
}

#[allow(clippy::expect_used)]
fn validate_policy_fixture_file() -> Result<PolicyFixtureStats, String> {
    #[cfg(all(test, feature = "coverage-controls"))]
    if FORCED_FIXTURE_FAILURE
        .compare_exchange(4, 0, Ordering::Relaxed, Ordering::Relaxed)
        .is_ok()
    {
        return Err(String::from("injected policy fixture failure"));
    }
    let source = read_text(DEFAULT_POLICY_FIXTURES)
        .expect("checked-in policy-ratchet fixture source is readable");
    validate_policy_fixture_source(&source)
}

fn validate_disposition_fixture_source(source: &str) -> Result<DispositionFixtureStats, String> {
    if !source.ends_with('\n') {
        return Err(String::from(
            "disposition-protocol fixtures must end with a newline",
        ));
    }
    let mut lines = source.lines();
    if lines.next() != Some(DISPOSITION_FIXTURE_HEADER) {
        return Err(format!(
            "disposition-protocol fixtures require exact header `{DISPOSITION_FIXTURE_HEADER}`"
        ));
    }
    let mut ids = BTreeSet::new();
    let mut positive_kinds = BTreeSet::new();
    let mut mutations = BTreeMap::<String, BTreeSet<String>>::new();
    let mut previous_id: Option<String> = None;
    for (offset, line) in lines.enumerate() {
        let line_number = offset + 2;
        let fields = line.split('\t').collect::<Vec<_>>();
        if fields.len() != 10 || fields[1..].contains(&"") {
            return Err(format!(
                "{DEFAULT_DISPOSITION_FIXTURES}:{line_number}: malformed closed fixture row"
            ));
        }
        if fields[0] != DISPOSITION_FIXTURE_SCHEMA {
            return Err(format!(
                "{DEFAULT_DISPOSITION_FIXTURES}:{line_number}: unknown fixture schema"
            ));
        }
        if previous_id
            .as_ref()
            .is_some_and(|previous| previous.as_str() > fields[1])
        {
            return Err(format!(
                "{DEFAULT_DISPOSITION_FIXTURES}:{line_number}: case IDs are not strictly sorted"
            ));
        }
        previous_id = Some(fields[1].to_owned());
        if !ids.insert(fields[1].to_owned()) {
            return Err(format!(
                "{DEFAULT_DISPOSITION_FIXTURES}:{line_number}: duplicate case ID"
            ));
        }
        let kind = fields[3];
        if !REQUIRED_DISPOSITION_KINDS.contains(&kind) {
            return Err(format!(
                "{DEFAULT_DISPOSITION_FIXTURES}:{line_number}: unknown disposition kind"
            ));
        }
        match (fields[2], fields[4], fields[7], fields[8]) {
            ("positive", "none", "accept", "-") => {
                positive_kinds.insert(kind.to_owned());
            }
            ("negative", mutation, "reject", diagnostic)
                if REQUIRED_DISPOSITION_MUTATIONS.contains(&mutation)
                    && diagnostic.starts_with("CHG-DISPOSITION-") =>
            {
                mutations
                    .entry(kind.to_owned())
                    .or_default()
                    .insert(mutation.to_owned());
            }
            _ => {
                return Err(format!(
                    "{DEFAULT_DISPOSITION_FIXTURES}:{line_number}: polarity, mutation, outcome, and diagnostic disagree"
                ));
            }
        }
    }
    let required_mutations = REQUIRED_DISPOSITION_MUTATIONS
        .iter()
        .map(|value| (*value).to_owned())
        .collect::<BTreeSet<_>>();
    let mut required_ids = BTreeSet::new();
    for kind in REQUIRED_DISPOSITION_KINDS {
        required_ids.insert(format!("disposition-{kind}-complete"));
        for mutation in REQUIRED_DISPOSITION_MUTATIONS {
            required_ids.insert(format!("disposition-{kind}-invalid-{mutation}"));
        }
        if !positive_kinds.contains(kind) || mutations.get(kind) != Some(&required_mutations) {
            return Err(format!(
                "disposition `{kind}` requires one complete case and every independent mutation"
            ));
        }
    }
    if ids != required_ids {
        return Err(String::from(
            "disposition-protocol fixture IDs differ from the reviewed closed inventory",
        ));
    }
    Ok(DispositionFixtureStats {
        cases: ids.len(),
        kinds: REQUIRED_DISPOSITION_KINDS.len(),
        mutations_per_kind: REQUIRED_DISPOSITION_MUTATIONS.len(),
    })
}

#[allow(clippy::expect_used)]
fn validate_disposition_fixture_file() -> Result<DispositionFixtureStats, String> {
    #[cfg(all(test, feature = "coverage-controls"))]
    if FORCED_FIXTURE_FAILURE
        .compare_exchange(5, 0, Ordering::Relaxed, Ordering::Relaxed)
        .is_ok()
    {
        return Err(String::from("injected disposition fixture failure"));
    }
    let source = read_text(DEFAULT_DISPOSITION_FIXTURES)
        .expect("checked-in disposition fixture source is readable");
    validate_disposition_fixture_source(&source)
}

/// Monomorphization-stable core of the schema API binding audit: taking a
/// slice (rather than a `[(&str, &str); N]` const-generic array reference)
/// means every caller shares this single non-generic body and its coverage
/// regions, regardless of how many bindings each caller's array literal has.
fn audit_api_bindings(
    rows: &[Row],
    category: &str,
    bindings: &[(&str, &str)],
    implementation: &str,
) -> Result<usize, String> {
    let declared = rows
        .iter()
        .filter(|row| row.category == category)
        .map(|row| row.name.as_str())
        .collect::<BTreeSet<_>>();
    let mapped = bindings
        .iter()
        .map(|(name, _)| *name)
        .collect::<BTreeSet<_>>();
    if declared != mapped {
        return Err(format!(
            "schema API {category} bindings differ from the frozen inventory"
        ));
    }
    for (name, anchor) in bindings {
        if !implementation.contains(anchor) {
            return Err(format!(
                "schema API {category} `{name}` is missing implementation anchor `{anchor}`"
            ));
        }
    }
    Ok(bindings.len())
}

fn braced_block_from<'a>(implementation: &'a str, marker: &str) -> Option<&'a str> {
    let start = implementation.find(marker)?;
    let block = &implementation[start..];
    let mut depth = 0_u64;
    for (offset, character) in block.char_indices() {
        match character {
            '{' => depth += 1,
            '}' => {
                depth = depth.checked_sub(1)?;
                if depth == 0 {
                    return block.get(..=offset);
                }
            }
            _ => {}
        }
    }
    None
}

fn implementation_block<'a>(implementation: &'a str, type_name: &str) -> Option<&'a str> {
    braced_block_from(implementation, &format!("impl {type_name} {{"))
}

fn pascal_case(value: &str) -> String {
    match value {
        "xxh3-128" => return String::from("Xxh3_128"),
        "blake3-256" => return String::from("Blake3_256"),
        "sha-256" => return String::from("Sha256"),
        _ => {}
    }
    let mut result = String::new();
    for component in value.split('-') {
        let mut characters = component.chars();
        if let Some(first) = characters.next() {
            result.extend(first.to_uppercase());
            result.extend(characters);
        }
    }
    result
}

fn declared_rust_variants(declaration: &str) -> BTreeSet<String> {
    declaration
        .lines()
        .filter_map(|line| {
            let (name, value) = line.trim().split_once('=')?;
            let name = name.trim();
            let value = value.trim();
            (!name.is_empty()
                && name
                    .chars()
                    .all(|character| character.is_ascii_alphanumeric() || character == '_')
                && value.as_bytes().first().is_some_and(u8::is_ascii_digit))
            .then(|| name.to_owned())
        })
        .collect()
}

#[allow(clippy::expect_used)]
fn audit_enum_variant_tags(rows: &[Row], implementation: &str) -> Result<(), String> {
    let bindings = ENUM_API_BINDINGS
        .iter()
        .copied()
        .collect::<BTreeMap<_, _>>();
    let mut expected_by_anchor = BTreeMap::<&str, BTreeSet<String>>::new();
    for row in rows.iter().filter(|row| row.category == "enum-variant") {
        let anchor = bindings
            .get(row.parent.as_str())
            .ok_or_else(|| format!("enum `{}` has no public API type binding", row.parent))?;
        let declaration = braced_block_from(implementation, anchor)
            .ok_or_else(|| format!("enum `{}` public API declaration is missing", row.parent))?;
        let tag = row
            .tag
            .ok_or_else(|| format!("enum variant `{}/{}` has no tag", row.parent, row.name))?;
        let variant = pascal_case(&row.name);
        if !declaration.contains(&format!("{variant} = {tag}")) {
            return Err(format!(
                "enum variant `{}/{}` does not match public API variant `{variant}` tag {tag}",
                row.parent, row.name
            ));
        }
        expected_by_anchor
            .entry(anchor)
            .or_default()
            .insert(variant);
    }
    for (anchor, expected) in expected_by_anchor {
        let declaration = braced_block_from(implementation, anchor)
            .expect("each collected enum anchor had a declaration above");
        let actual = declared_rust_variants(declaration);
        if actual != expected {
            return Err(format!(
                "public enum declaration `{anchor}` variants differ from the frozen inventory"
            ));
        }
    }
    Ok(())
}

fn audit_object_kind_tags(rows: &[Row], implementation: &str) -> Result<(), String> {
    let declaration = braced_block_from(implementation, "pub enum ObjectKind")
        .ok_or_else(|| String::from("object-kind public API declaration is missing"))?;
    let mut expected = BTreeSet::new();
    for row in rows.iter().filter(|row| row.category == "object") {
        let tag = row
            .tag
            .ok_or_else(|| format!("object `{}` has no tag", row.name))?;
        let variant = pascal_case(&row.name);
        if !declaration.contains(&format!("{variant} = {tag}")) {
            return Err(format!(
                "object `{}` does not match ObjectKind::{variant} tag {tag}",
                row.name
            ));
        }
        expected.insert(variant);
    }
    if declared_rust_variants(declaration) != expected {
        return Err(String::from(
            "ObjectKind variants differ from the frozen object inventory",
        ));
    }
    Ok(())
}

fn has_validator(name: &str, block: &str, implementation: &str) -> bool {
    if name == "snapshot-budgets" {
        block.contains("fn validate(")
    } else if name == "oracle-command" {
        implementation.contains("for oracle in &config.oracles")
            && implementation.contains("#[serde(deny_unknown_fields)]")
    } else if name == "mounted-selector" {
        implementation.contains("fn parse_selectors(")
    } else if matches!(
        name,
        "logical-mount" | "source-child-reference" | "source-edge" | "source-budgets"
    ) {
        block.contains("pub const fn new(")
    } else {
        block.contains("pub fn new(")
    }
}

/// Monomorphization-stable core of the schema API validator audit: see
/// [`audit_api_bindings`] for why a slice parameter (instead of a
/// `[(&str, &str); N]` const-generic array reference) keeps this body's
/// coverage regions shared across every caller.
fn audit_validators(
    category: &str,
    bindings: &[(&str, &str)],
    implementation: &str,
) -> Result<(), String> {
    for (name, anchor) in bindings {
        if *name == "source-item-kind" {
            if implementation.contains("SourceItemKind,\n    versioned_lowercase_id_grammar")
                && implementation.contains("macro_rules! identifier_newtype")
                && implementation.contains("pub fn new(input: &str)")
            {
                continue;
            }
            return Err(format!(
                "schema API {category} `{name}` has no validated macro construction anchor"
            ));
        }
        let type_name = anchor
            .split_whitespace()
            .last()
            .ok_or_else(|| format!("schema API {category} `{name}` has an empty type anchor"))?;
        let block = implementation_block(implementation, type_name).ok_or_else(|| {
            format!("schema API {category} `{name}` has no concrete implementation block")
        })?;
        if !has_validator(name, block, implementation) {
            return Err(format!(
                "schema API {category} `{name}` has no validated construction anchor"
            ));
        }
    }
    Ok(())
}

fn validate_canonical_vector_parity(rows: &[Row], source: &str) -> Result<usize, String> {
    if !source.ends_with('\n') {
        return Err(String::from("canonical vectors must end with a newline"));
    }
    let mut lines = source.lines();
    if lines.next() != Some(CANONICAL_VECTOR_HEADER) {
        return Err(format!(
            "canonical vectors require exact header `{CANONICAL_VECTOR_HEADER}`"
        ));
    }
    let mut actual = BTreeSet::new();
    for (offset, line) in lines.enumerate() {
        let line_number = offset + 2;
        let fields = line.split('\t').collect::<Vec<_>>();
        if fields.len() != 5 || fields[0] != "spec-provenance-canonical-vector/v2" {
            return Err(format!(
                "{DEFAULT_CANONICAL_VECTORS}:{line_number}: malformed canonical vector row"
            ));
        }
        if !actual.insert((fields[1].to_owned(), fields[2].to_owned())) {
            return Err(format!(
                "{DEFAULT_CANONICAL_VECTORS}:{line_number}: duplicate kind/ID"
            ));
        }
    }
    let expected = rows
        .iter()
        .filter(|row| {
            matches!(
                row.category.as_str(),
                "type-tag" | "nested" | "domain" | "report-fact"
            )
        })
        .map(|row| (row.category.clone(), row.name.clone()))
        .collect::<BTreeSet<_>>();
    if actual != expected {
        return Err(String::from(
            "canonical vector kind/ID rows differ from the complete API inventory",
        ));
    }
    Ok(actual.len())
}

fn validate_api_parity_sources(
    rows: &[Row],
    implementation: &str,
    canonical_vectors: &str,
) -> Result<ApiParityStats, String> {
    let enums = audit_api_bindings(rows, "enum", &ENUM_API_BINDINGS, implementation)?;
    let objects = audit_api_bindings(rows, "object", &OBJECT_API_BINDINGS, implementation)?;
    audit_enum_variant_tags(rows, implementation)?;
    audit_object_kind_tags(rows, implementation)?;
    let nested = audit_api_bindings(rows, "nested", &NESTED_API_BINDINGS, implementation)?;
    let report_facts =
        audit_api_bindings(rows, "report-fact", &REPORT_API_BINDINGS, implementation)?;
    audit_validators("object", &OBJECT_API_BINDINGS, implementation)?;
    audit_validators("nested", &NESTED_API_BINDINGS, implementation)?;
    audit_validators("report-fact", &REPORT_API_BINDINGS, implementation)?;
    let domains = audit_api_bindings(rows, "domain", &DOMAIN_API_BINDINGS, implementation)?;
    for (domain, anchor) in DOMAIN_API_BINDINGS {
        let registration = format!("\"{domain}\"");
        let implementation_anchor = if let Some(name) = anchor.strip_prefix("pub struct ") {
            format!("impl {name} {{")
        } else if anchor.starts_with("impl ") {
            format!("{anchor} {{")
        } else {
            String::from(anchor)
        };
        let encoder_block = braced_block_from(implementation, &implementation_anchor).ok_or_else(|| {
            format!(
                "schema API domain `{domain}` has no production encoder block `{implementation_anchor}`"
            )
        })?;
        if !encoder_block.contains(&registration) {
            return Err(format!(
                "schema API domain `{domain}` has no production encoder registration in `{anchor}`"
            ));
        }
    }
    let canonical_vectors = validate_canonical_vector_parity(rows, canonical_vectors)?;
    Ok(ApiParityStats {
        enums,
        objects,
        nested,
        domains,
        report_facts,
        canonical_vectors,
    })
}

#[allow(clippy::expect_used)]
fn validate_api_parity(rows: &[Row]) -> Result<ApiParityStats, String> {
    #[cfg(all(test, feature = "coverage-controls"))]
    if FORCED_FIXTURE_FAILURE
        .compare_exchange(6, 0, Ordering::Relaxed, Ordering::Relaxed)
        .is_ok()
    {
        return Err(String::from("injected schema API parity failure"));
    }
    let mut implementation = String::new();
    for path in IMPLEMENTATION_SOURCES {
        let source = read_text(path).expect("checked-in implementation source is readable");
        implementation.push_str(&source);
        implementation.push('\n');
    }
    let canonical_vectors = read_text(DEFAULT_CANONICAL_VECTORS)
        .expect("checked-in canonical vector source is readable");
    validate_api_parity_sources(rows, &implementation, &canonical_vectors)
}

fn without_declaration(
    source: &str,
    category: &str,
    name: &str,
    reference_replacement: Option<(&str, &str)>,
) -> String {
    let mut result = source
        .lines()
        .filter(|line| {
            let columns = line.split('\t').collect::<Vec<_>>();
            columns.first() != Some(&category) || columns.get(2) != Some(&name)
        })
        .map(|line| {
            reference_replacement
                .map_or_else(|| line.to_owned(), |(from, to)| line.replace(from, to))
        })
        .collect::<Vec<_>>()
        .join("\n");
    result.push('\n');
    result
}

#[derive(Default)]
struct SelfTestChecks {
    first_failure: Option<String>,
}

impl SelfTestChecks {
    fn require(&mut self, condition: bool, message: &str) {
        if !condition && self.first_failure.is_none() {
            self.first_failure = Some(message.to_owned());
        }
    }

    fn finish(self) -> Result<(), String> {
        self.first_failure.map_or(Ok(()), Err)
    }
}

#[allow(clippy::expect_used)]
fn self_test() -> Result<(), String> {
    let mut injected_checks = SelfTestChecks::default();
    injected_checks.require(true, "accepted control");
    injected_checks.require(false, "rejected control");
    injected_checks.require(false, "retained first rejection");
    drop(injected_checks.finish());
    let mut checks = SelfTestChecks::default();
    let source =
        read_text(DEFAULT_INVENTORY).expect("checked-in schema inventory source is readable");
    let positive = validate(&source).map(|_| ());

    drop(validate(source.trim_end_matches('\n')));
    drop(validate(&source.replacen(HEADER, "wrong-header", 1)));
    drop(validate(&format!("{HEADER}\n")));
    drop(validate(&format!(
        "{HEADER}\nunknown\t-\tx\t-\tx\trequired\tscalar\tx\n"
    )));
    drop(validate(&format!("{HEADER}\nfield\ttoo-few\n")));
    drop(validate(&format!(
        "{source}{}\n",
        source.lines().nth(1).unwrap_or_default()
    )));
    drop(validate(&source.replacen(
        "schema\t-\tspec-provenance/v3\t3\tschema\trequired\tscalar\tfrozen semantic schema\n",
        "",
        1,
    )));
    drop(validate(&source.replacen(
        "enum-variant\tdigest-algorithm\txxh3-128\t2\tunit\trequired\tscalar\t16 bytes; change-detection; seed 0; standard default secret; high64-be then low64-be\n",
        "",
        1,
    )));
    drop(validate(&source.replacen(
        "type-tag\tcanonical-value\tdigest\t12\tdigest\trequired\tscalar\tu16 algorithm u16 length bytes\n",
        "",
        1,
    )));
    drop(validate(&format!(
        "{source}object\tobjects\textra\t21\tdomain:atom-object\trequired\tscalar\tcount control\n"
    )));
    for malformed_row in [
        "",
        "field\ttoo-few",
        "field\tdomain\tname\tnot-u16\tshape\trequired\tscalar\trule",
        "unknown\t-\tx\t-\tx\trequired\tscalar\tx",
    ] {
        drop(parse_row(2, malformed_row));
    }
    for replacement in [
        "schema\t-\tspec-provenance/v3\t3\t\trequired\tscalar\tfrozen semantic schema",
        "schema\t-\tspec-provenance/v3\t3\tschema\t\tscalar\tfrozen semantic schema",
        "schema\t-\tspec-provenance/v3\t3\tschema\trequired\t\tfrozen semantic schema",
        "schema\t-\tspec-provenance/v3\t3\tschema\trequired\tscalar\t",
        "schema\t-\twrong-schema\t3\tschema\trequired\tscalar\tfrozen semantic schema",
        "schema\t-\tspec-provenance/v3\t2\tschema\trequired\tscalar\tfrozen semantic schema",
    ] {
        drop(validate(&source.replacen(
            source.lines().nth(1).unwrap_or_default(),
            replacement,
            1,
        )));
    }
    let digest = "enum-variant\tdigest-algorithm\txxh3-128\t2\tunit\trequired\tscalar\t16 bytes; change-detection; seed 0; standard default secret; high64-be then low64-be";
    for replacement in [
        "enum-variant\tdigest-algorithm\twrong\t2\tunit\trequired\tscalar\t16 bytes; change-detection; seed 0; standard default secret; high64-be then low64-be",
        "enum-variant\tdigest-algorithm\txxh3-128\t3\tunit\trequired\tscalar\t16 bytes; change-detection; seed 0; standard default secret; high64-be then low64-be",
        "enum-variant\tdigest-algorithm\txxh3-128\t2\tunit\trequired\tscalar\twrong",
        "enum-variant\tdigest-algorithm\txxh3-128\t2\tunit\trequired\tscalar\t16 bytes but wrong order",
    ] {
        drop(validate(&source.replacen(digest, replacement, 1)));
    }
    let swapped_digest_tags = source
        .replacen("\txxh3-128\t2\t", "\txxh3-128\t3\t", 1)
        .replacen("\tblake3-256\t3\t", "\tblake3-256\t2\t", 1);
    drop(validate(&swapped_digest_tags));
    drop(validate(&source.replacen(
        "object\tobjects\tatom\t1\tdomain:atom-object\trequired\tscalar\tkind-specific validated construction\n",
        "",
        1,
    )));
    drop(validate(&source.replacen(
        "field\tobject-reference\tobject-kind",
        "field\tmissing-parent\tobject-kind",
        1,
    )));
    let mut missing_required_domain = without_declaration(
        &source,
        "domain",
        "raw-artifact",
        Some(("domain:raw-artifact", "domain:repository-config")),
    );
    missing_required_domain
        .push_str("nested\t-\traw-artifact\t-\tstruct\trequired\tscalar\tparent control\n");
    drop(validate(&missing_required_domain));
    let mut missing_additional_domain = without_declaration(
        &source,
        "domain",
        "change-disposition",
        Some(("domain:change-disposition", "domain:impact-set")),
    );
    missing_additional_domain
        .push_str("nested\t-\tchange-disposition\t-\tstruct\trequired\tscalar\tparent control\n");
    drop(validate(&missing_additional_domain));
    let mut missing_report_fact =
        without_declaration(&source, "report-fact", "authority-fact", None);
    missing_report_fact
        .push_str("nested\t-\tauthority-fact\t-\tstruct\trequired\tscalar\tparent control\n");
    drop(validate(&missing_report_fact));
    drop(validate(&without_declaration(
        &source,
        "diagnostic-family",
        "SPG-AUTH-*",
        None,
    )));
    let without_entry_variants = source
        .lines()
        .filter(|line| !line.starts_with("enum-variant\tentry-kind\t"))
        .collect::<Vec<_>>()
        .join("\n")
        + "\n";
    drop(validate(&without_entry_variants));

    let row = |category: &str, parent: &str, shape: &str| Row {
        line: 2,
        category: category.to_owned(),
        parent: parent.to_owned(),
        name: String::from("test"),
        tag: Some(1),
        shape: shape.to_owned(),
        cardinality: String::from("required"),
        collection: String::from("scalar"),
        rule: String::from("test"),
    };
    let declarations = BTreeMap::new();
    drop(check_declared_parents(
        &[row("field", "missing", "scalar")],
        &declarations,
    ));
    drop(check_declared_parents(
        &[row("enum-variant", "missing", "unit")],
        &declarations,
    ));
    drop(check_declared_parents(
        &[row("object", "objects", "wrong")],
        &declarations,
    ));
    drop(check_declared_parents(
        &[row("object", "objects", "domain:missing")],
        &declarations,
    ));
    drop(check_declared_parents(
        &[row("object", "objects", "domain:change-disposition")],
        &declarations,
    ));
    let mut enum_declarations = BTreeMap::new();
    enum_declarations.insert(
        String::from("enum"),
        BTreeSet::from([String::from("missing-variants")]),
    );
    drop(check_enum_variants(&[], &enum_declarations));

    let mut tags = BTreeMap::new();
    tags.insert(
        (String::from("field"), String::from("duplicate")),
        vec![(1, 2), (1, 3)],
    );
    drop(check_tag_sequences(&tags));
    tags.insert(
        (String::from("field"), String::from("gap")),
        vec![(1, 2), (3, 3)],
    );
    drop(check_tag_sequences(&tags));
    drop(check_exact_tags(&tags, "missing", "group", 0, 1));
    tags.insert(
        (String::from("type-tag"), String::from("canonical-value")),
        vec![(0, 2)],
    );
    drop(check_exact_tags(
        &tags,
        "type-tag",
        "canonical-value",
        0,
        12,
    ));

    let empty = BTreeMap::new();
    drop(require_names(&empty, "domain", &REQUIRED_DOMAINS));
    drop(require_names(&empty, "domain", &ADDITIONAL_DOMAINS));
    drop(require_names(&empty, "report-fact", &REQUIRED_REPORT_FACTS));
    drop(require_names(
        &empty,
        "diagnostic-family",
        &REQUIRED_DIAGNOSTIC_FAMILIES,
    ));

    positive.expect("checked-in schema inventory source is valid");
    let rows = validated_rows(&source).expect("validated inventory rows remain parseable");
    let api_stats = validate_api_parity(&rows).expect("schema API parity validates");
    checks.require(
        ![
            api_stats.enums,
            api_stats.objects,
            api_stats.nested,
            api_stats.domains,
            api_stats.report_facts,
            api_stats.canonical_vectors,
        ]
        .contains(&0),
        "schema API parity omitted a required class",
    );
    let mut implementation = String::new();
    for implementation_path in IMPLEMENTATION_SOURCES {
        implementation.push_str(
            &read_text(implementation_path).expect("checked-in implementation source is readable"),
        );
        implementation.push('\n');
    }
    let canonical_vectors = read_text(DEFAULT_CANONICAL_VECTORS)
        .expect("checked-in canonical vector source is readable");
    let missing_type = implementation.replacen("pub struct Atom", "struct Atom", 1);
    let missing_validator =
        implementation.replacen("impl Atom {", "impl AtomWithoutValidator {", 1);
    let mismatched_enum_tag = implementation.replacen("Behavioral = 1", "Behavioral = 99", 1);
    let mismatched_object_tag = implementation.replacen("Atom = 1", "Atom = 99", 1);
    let missing_encoder = implementation.replacen(
        "\"artifact-index\"",
        "\"artifact-index-negative-control\"",
        1,
    );
    let first_vector = canonical_vectors
        .lines()
        .nth(1)
        .expect("validated canonical vector matrix has cases");
    let missing_vector = canonical_vectors.replacen(&format!("{first_vector}\n"), "", 1);
    let rows_without_object = rows
        .iter()
        .filter(|row| !(row.category == "object" && row.name == "atom"))
        .cloned()
        .collect::<Vec<_>>();
    for (label, result) in [
        (
            "missing public type",
            validate_api_parity_sources(&rows, &missing_type, &canonical_vectors),
        ),
        (
            "missing validator",
            validate_api_parity_sources(&rows, &missing_validator, &canonical_vectors),
        ),
        (
            "mismatched enum tag",
            validate_api_parity_sources(&rows, &mismatched_enum_tag, &canonical_vectors),
        ),
        (
            "mismatched object tag",
            validate_api_parity_sources(&rows, &mismatched_object_tag, &canonical_vectors),
        ),
        (
            "missing encoder",
            validate_api_parity_sources(&rows, &missing_encoder, &canonical_vectors),
        ),
        (
            "missing canonical fixture row",
            validate_api_parity_sources(&rows, &implementation, &missing_vector),
        ),
        (
            "missing schema type row",
            validate_api_parity_sources(&rows_without_object, &implementation, &canonical_vectors),
        ),
    ] {
        checks.require(
            result.is_err(),
            &format!("schema API parity {label} control was accepted"),
        );
    }
    let (expected_fixtures, expected_stats) = render_construction_fixtures(&rows);
    let actual_fixtures = read_text(DEFAULT_CONSTRUCTION_FIXTURES)
        .expect("checked-in construction fixture source is readable");
    validate_construction_fixture_source(&actual_fixtures, &expected_fixtures)
        .expect("checked-in construction fixture source is current");
    let first_case = expected_fixtures
        .lines()
        .nth(1)
        .expect("validated schema-construction fixture matrix has cases");
    let missing_case = expected_fixtures.replacen(&format!("{first_case}\n"), "", 1);
    let duplicate_case = format!("{expected_fixtures}{first_case}\n");
    let wrong_outcome =
        expected_fixtures.replacen("\taccept\t-\n", "\treject\tSCF-CONSTRUCT-CONTROL\n", 1);
    for (label, control) in [
        ("missing case", missing_case),
        ("duplicate case", duplicate_case),
        ("wrong outcome", wrong_outcome),
    ] {
        checks.require(
            validate_construction_fixture_source(&control, &expected_fixtures).is_err(),
            &format!("schema-construction {label} negative control was accepted"),
        );
    }
    checks.require(
        audit_construction_fixture_coverage(&rows, &BTreeMap::new()).is_err(),
        "schema-construction coverage audit accepted an empty case set",
    );
    checks.require(
        ![
            expected_stats.fields,
            expected_stats.option_fields,
            expected_stats.empty_forms,
            expected_stats.enum_variants,
            expected_stats.enums,
            expected_stats.reference_kinds,
            expected_stats.objects,
            expected_stats.terminal_records,
        ]
        .contains(&0),
        "schema-construction fixtures omit a required coverage class",
    );
    let mut duplicate_cases = BTreeMap::new();
    checks.require(
        insert_construction_case(
            &mut duplicate_cases,
            String::from("duplicate-control"),
            "positive",
            "field",
            "control",
            "control",
            "construct-field",
            "present-valid",
            "none",
            "accept",
            "-",
        )
        .is_ok(),
        "initial schema-construction fixture ID control was rejected",
    );
    checks.require(
        insert_construction_case(
            &mut duplicate_cases,
            String::from("duplicate-control"),
            "negative",
            "field",
            "control",
            "control",
            "construct-field",
            "omitted",
            "omit-field",
            "reject",
            "SCF-CONSTRUCT-FIELD-MISSING",
        )
        .is_err(),
        "duplicate schema-construction fixture ID control was accepted",
    );

    let state_source = read_text(DEFAULT_STATE_FIXTURES)
        .expect("checked-in schema-state fixture source is readable");
    validate_state_fixture_source(&state_source)
        .expect("checked-in schema-state fixture source is valid");
    let state_first_case = state_source
        .lines()
        .nth(1)
        .expect("validated schema-state fixture matrix has cases");
    let state_controls = [
        state_source.replacen(&format!("{state_first_case}\n"), "", 1),
        format!("{state_source}{state_first_case}\n"),
        state_source.replacen(STATE_FIXTURE_SCHEMA, "wrong-schema", 1),
        state_source.replacen("\tpositive\t", "\tunknown\t", 1),
        state_source.replacen("\taccept\t-\t", "\treject\t-\t", 1),
        state_source.replacen("\taccept\t-\t", "\taccept\tSCF-WRONG\t", 1),
        state_source.replacen("approval-valid=true;record-current=true", "", 1),
        state_source.replacen("\tapproval-binding\t", "\tunknown-subject\t", 1),
    ];
    for control in state_controls {
        checks.require(
            validate_state_fixture_source(&control).is_err(),
            "malformed schema-state fixture negative control was accepted",
        );
    }

    for malformed_inventory in [
        source.replacen("target:atom-object", "wrong-target-shape", 1),
        source.replacen("source-field:replacements", "wrong-source-rule", 1),
    ] {
        checks.require(
            validate(&malformed_inventory).is_err(),
            "malformed impact-edge inventory negative control was accepted",
        );
    }
    let (expected_impact_fixtures, impact_stats) =
        render_impact_fixtures(&rows).expect("validated inventory renders impact fixtures");
    let actual_impact_fixtures =
        read_text(DEFAULT_IMPACT_FIXTURES).expect("checked-in impact fixture source is readable");
    validate_impact_fixture_source(&actual_impact_fixtures, &expected_impact_fixtures)
        .expect("checked-in impact fixture source is current");
    checks.require(
        (impact_stats.edges != 0, impact_stats.chains >= 3) == (true, true),
        "impact-edge fixtures omit declared edges or multi-hop chains",
    );
    let first_impact_case = expected_impact_fixtures
        .lines()
        .nth(1)
        .expect("validated impact-edge fixture matrix has cases");
    for control in [
        expected_impact_fixtures.replacen(&format!("{first_impact_case}\n"), "", 1),
        format!("{expected_impact_fixtures}{first_impact_case}\n"),
        expected_impact_fixtures.replacen("\taccept\t-\t", "\treject\t-\t", 1),
    ] {
        checks.require(
            validate_impact_fixture_source(&control, &expected_impact_fixtures).is_err(),
            "malformed impact-edge fixture negative control was accepted",
        );
    }
    let rows_without_edges = rows
        .iter()
        .filter(|row| row.category != "impact-edge")
        .cloned()
        .collect::<Vec<_>>();
    checks.require(
        render_impact_fixtures(&rows_without_edges).is_err(),
        "impact-edge fixture renderer accepted an empty edge inventory",
    );
    let rows_without_chain_edge = rows
        .iter()
        .filter(|row| {
            !(row.category == "impact-edge"
                && row.parent == "formal-property"
                && row.name == "atoms")
        })
        .cloned()
        .collect::<Vec<_>>();
    checks.require(
        render_impact_fixtures(&rows_without_chain_edge).is_err(),
        "impact-edge fixture renderer accepted an undeclared chain edge",
    );

    let policy_source = read_text(DEFAULT_POLICY_FIXTURES)
        .expect("checked-in policy-ratchet fixture source is readable");
    validate_policy_fixture_source(&policy_source)
        .expect("checked-in policy-ratchet fixture source is valid");
    let policy_first_case = policy_source
        .lines()
        .nth(1)
        .expect("validated policy-ratchet fixture matrix has cases");
    for control in [
        policy_source.replacen(&format!("{policy_first_case}\n"), "", 1),
        format!("{policy_source}{policy_first_case}\n"),
        policy_source.replacen(POLICY_FIXTURE_SCHEMA, "wrong-schema", 1),
        policy_source.replacen("\tauthority\t", "\tunknown-dimension\t", 1),
        policy_source.replacen("\tweakening\treject\t", "\tstrengthening\treject\t", 1),
        policy_source.replacen("\tpositive\t", "\tnegative\t", 1),
    ] {
        checks.require(
            validate_policy_fixture_source(&control).is_err(),
            "malformed policy-ratchet fixture negative control was accepted",
        );
    }

    let disposition_source = read_text(DEFAULT_DISPOSITION_FIXTURES)
        .expect("checked-in disposition fixture source is readable");
    validate_disposition_fixture_source(&disposition_source)
        .expect("checked-in disposition fixture source is valid");
    let disposition_first_case = disposition_source
        .lines()
        .nth(1)
        .expect("validated disposition-protocol fixture matrix has cases");
    for control in [
        disposition_source.replacen(&format!("{disposition_first_case}\n"), "", 1),
        format!("{disposition_source}{disposition_first_case}\n"),
        disposition_source.replacen(DISPOSITION_FIXTURE_SCHEMA, "wrong-schema", 1),
        disposition_source.replacen("\treaffirmed\t", "\tunknown-kind\t", 1),
        disposition_source.replacen("\tbase-digest\t", "\tunknown-mutation\t", 1),
        disposition_source.replacen("\tnegative\t", "\tpositive\t", 1),
        disposition_source.replacen(
            "\treject\tCHG-DISPOSITION-",
            "\taccept\tCHG-DISPOSITION-",
            1,
        ),
    ] {
        checks.require(
            validate_disposition_fixture_source(&control).is_err(),
            "malformed disposition-protocol fixture negative control was accepted",
        );
    }

    // -- Coverage controls below exercise branches that only the plain
    //    (non `--self-test`) invocation or `cargo test`-only fixtures would
    //    otherwise reach, so this self-test drives them directly.
    checks.require(
        read_text("schema-inventory-self-test-missing-file.tsv").is_err(),
        "read_text accepted a missing path",
    );
    let write_scratch_path = std::env::temp_dir()
        .join("schema-inventory-self-test-write-text.tmp")
        .to_string_lossy()
        .into_owned();
    write_text(
        &write_scratch_path,
        "self-test write_text success control\n",
    )
    .expect("write_text can write a scratch file");
    checks.require(
        read_text(&write_scratch_path).as_deref() == Ok("self-test write_text success control\n"),
        "write_text success control did not persist its content",
    );
    let _ = fs::remove_file(&write_scratch_path);
    checks.require(
        write_text(".", "self-test write_text failure control").is_err(),
        "write_text accepted a directory path",
    );

    // `run_with_arguments` argument-dispatch branches.
    checks.require(
        run_with_arguments(&[String::from(DEFAULT_INVENTORY)]).is_ok(),
        "run-with-arguments rejected the default inventory path passed explicitly",
    );
    checks.require(
        run_with_arguments(&[String::from("one"), String::from("two")]).is_err(),
        "run-with-arguments accepted more than one argument",
    );
    checks.require(
        run_with_arguments(&[String::from("schema-inventory-self-test-missing-file.tsv")]).is_err(),
        "run-with-arguments accepted a missing inventory path",
    );
    let malformed_inventory_path = std::env::temp_dir()
        .join("schema-inventory-self-test-malformed-inventory.tsv")
        .to_string_lossy()
        .into_owned();
    write_text(&malformed_inventory_path, "not-a-valid-schema-inventory\n")
        .expect("write malformed scratch inventory");
    checks.require(
        run_with_arguments(std::slice::from_ref(&malformed_inventory_path)).is_err(),
        "run-with-arguments accepted a malformed inventory path",
    );
    let _ = fs::remove_file(&malformed_inventory_path);
    let alternate_inventory_path = std::env::temp_dir()
        .join("schema-inventory-self-test-alternate-inventory.tsv")
        .to_string_lossy()
        .into_owned();
    write_text(&alternate_inventory_path, &source).expect("write alternate scratch inventory");
    checks.require(
        run_with_arguments(std::slice::from_ref(&alternate_inventory_path)).is_ok(),
        "run-with-arguments rejected a valid non-default inventory path",
    );
    let _ = fs::remove_file(&alternate_inventory_path);
    // The self-test must not write or corrupt repository files. The explicit
    // coverage lane runs both write commands after the parallel test process
    // exits, so Nix source snapshots cannot observe transient file content.

    // `exit_code`, extracted from `main` so both branches are reachable
    // without terminating the self-test process.
    checks.require(
        exit_code(Ok(())) == 0,
        "exit_code rejected a successful run",
    );
    checks.require(
        exit_code(Err(String::from("self-test exit_code control"))) == 1,
        "exit_code accepted a failed run",
    );

    // `evidence_bindings` and `render_execution_evidence_for`, extracted
    // from `render_execution_evidence` so a broken path or fixture list can
    // be exercised without disturbing the checked-in default sources.
    checks.require(
        evidence_bindings(&["schema-inventory-self-test-missing-tool.rs"], &[]).is_err(),
        "evidence_bindings accepted a missing tool path",
    );
    checks.require(
        evidence_bindings(
            &[],
            &["schema-inventory-self-test-missing-configuration.rs"],
        )
        .is_err(),
        "evidence_bindings accepted a missing configuration path",
    );
    let broken_fixture = EvidenceFixture {
        path: "schema-inventory-self-test-missing-fixture.tsv",
        operation: None,
        mutation: None,
        result: &[],
    };
    checks.require(
        render_execution_evidence_for(&[broken_fixture], "tp", "td", "cp", "cd").is_err(),
        "render_execution_evidence_for accepted an unreadable fixture",
    );
    checks.require(
        render_execution_evidence_from(&[], &["schema-inventory-self-test-missing-tool.rs"], &[])
            .is_err(),
        "render_execution_evidence_from accepted a missing tool path",
    );
    checks.require(
        render_execution_evidence_from(
            &[broken_fixture],
            &EXECUTION_TOOL_PATHS,
            &EXECUTION_CONFIGURATION_PATHS,
        )
        .is_err(),
        "render_execution_evidence_from accepted an unreadable fixture",
    );

    // `validate_execution_evidence_with`, extracted from
    // `validate_execution_evidence` so a failed render or a materialized
    // mismatch can be exercised without writing to the checked-in evidence
    // file.
    checks.require(
        validate_execution_evidence_with(Err(String::from("self-test render failure")), false)
            .is_err(),
        "validate_execution_evidence_with accepted a failed render",
    );
    checks.require(
        validate_execution_evidence_with(Ok(String::from("stale-content\n")), false).is_err(),
        "validate_execution_evidence_with accepted stale materialized content",
    );

    // `fixture_execution_records`, exercised directly since the real
    // pipeline only ever reads well-formed checked-in fixture files.
    let missing_fixture = EvidenceFixture {
        path: "schema-inventory-self-test-missing-execution-fixture.tsv",
        operation: None,
        mutation: None,
        result: &[],
    };
    checks.require(
        fixture_execution_records(missing_fixture, "tp", "td", "cp", "cd").is_err(),
        "fixture_execution_records accepted an unreadable fixture",
    );
    let empty_fixture_path =
        leaked_scratch_path("schema-inventory-self-test-empty-execution-fixture.tsv");
    write_text(empty_fixture_path, "").expect("write empty scratch execution fixture");
    let empty_fixture = EvidenceFixture {
        path: empty_fixture_path,
        operation: None,
        mutation: None,
        result: &[],
    };
    checks.require(
        fixture_execution_records(empty_fixture, "tp", "td", "cp", "cd").is_err(),
        "fixture_execution_records accepted an empty fixture",
    );
    let missing_case_id_path =
        leaked_scratch_path("schema-inventory-self-test-missing-case-id-fixture.tsv");
    write_text(missing_case_id_path, "header\nno-tabs-here\n")
        .expect("write missing-case-id scratch execution fixture");
    let missing_case_id_fixture = EvidenceFixture {
        path: missing_case_id_path,
        operation: None,
        mutation: None,
        result: &[],
    };
    checks.require(
        fixture_execution_records(missing_case_id_fixture, "tp", "td", "cp", "cd").is_err(),
        "fixture_execution_records accepted a data row with no case ID column",
    );
    let missing_result_path =
        leaked_scratch_path("schema-inventory-self-test-missing-result-fixture.tsv");
    write_text(missing_result_path, "header\nschema\tCASE1\n")
        .expect("write missing-result scratch execution fixture");
    let missing_result_fixture = EvidenceFixture {
        path: missing_result_path,
        operation: None,
        mutation: None,
        result: &[4, 5],
    };
    checks.require(
        fixture_execution_records(missing_result_fixture, "tp", "td", "cp", "cd").is_err(),
        "fixture_execution_records accepted a data row missing a normalized result column",
    );
    let missing_operation_fixture = EvidenceFixture {
        path: missing_result_path,
        operation: Some(3),
        mutation: None,
        result: &[1],
    };
    checks.require(
        fixture_execution_records(missing_operation_fixture, "tp", "td", "cp", "cd").is_err(),
        "fixture_execution_records accepted a data row missing its operation column",
    );
    let missing_mutation_fixture = EvidenceFixture {
        path: missing_result_path,
        operation: None,
        mutation: Some(3),
        result: &[1],
    };
    checks.require(
        fixture_execution_records(missing_mutation_fixture, "tp", "td", "cp", "cd").is_err(),
        "fixture_execution_records accepted a data row missing its mutation column",
    );
    let evidence_build_fixture_path =
        leaked_scratch_path("schema-inventory-self-test-evidence-build-fixture.tsv");
    write_text(
        evidence_build_fixture_path,
        "header\nschema\tCASE1\top1\tmut1\tres1\tres2\n",
    )
    .expect("write evidence-build scratch execution fixture");
    let operation_build_fixture = EvidenceFixture {
        path: evidence_build_fixture_path,
        operation: Some(2),
        mutation: None,
        result: &[4, 5],
    };
    checks.require(
        fixture_execution_records(operation_build_fixture, "", "td", "cp", "cd").is_err(),
        "fixture_execution_records accepted an invalid tool-path binding on its operation branch",
    );
    let mutation_build_fixture = EvidenceFixture {
        path: evidence_build_fixture_path,
        operation: None,
        mutation: Some(3),
        result: &[4, 5],
    };
    checks.require(
        fixture_execution_records(mutation_build_fixture, "", "td", "cp", "cd").is_err(),
        "fixture_execution_records accepted an invalid tool-path binding on its mutation branch",
    );
    let skipped_mutation_fixture_path =
        leaked_scratch_path("schema-inventory-self-test-skipped-mutation-fixture.tsv");
    write_text(
        skipped_mutation_fixture_path,
        "header\nschema\tCASE1\tnone\tres1\n",
    )
    .expect("write skipped-mutation scratch execution fixture");
    let skipped_mutation_fixture = EvidenceFixture {
        path: skipped_mutation_fixture_path,
        operation: None,
        mutation: Some(2),
        result: &[1],
    };
    checks.require(
        fixture_execution_records(skipped_mutation_fixture, "tp", "td", "cp", "cd").is_ok(),
        "fixture_execution_records rejected a mutation column with no mutation applied",
    );
    let no_operation_no_mutation_fixture = EvidenceFixture {
        path: skipped_mutation_fixture_path,
        operation: None,
        mutation: None,
        result: &[1],
    };
    checks.require(
        fixture_execution_records(no_operation_no_mutation_fixture, "tp", "td", "cp", "cd").is_ok(),
        "fixture_execution_records rejected a fixture with neither an operation nor a mutation column",
    );
    for path in [
        empty_fixture_path,
        missing_case_id_path,
        missing_result_path,
        evidence_build_fixture_path,
        skipped_mutation_fixture_path,
    ] {
        let _ = fs::remove_file(path);
    }

    // `audit_construction_fixture_coverage` and `require_construction_case`:
    // corrupt exactly one real case ID's content at a time so every
    // call-site inside the coverage audit fails at a distinct location
    // while every row processed before it still validates cleanly.
    let mut construction_case_map = expected_fixtures
        .lines()
        .skip(1)
        .map(|line| {
            let id = line
                .split('\t')
                .nth(1)
                .expect("construction fixture case ID field")
                .to_owned();
            (id, format!("{line}\n"))
        })
        .collect::<BTreeMap<_, _>>();
    let truncate_columns = |line: &str| -> String {
        let fields = line.trim_end().split('\t').collect::<Vec<_>>();
        format!("{}\n", fields[..fields.len().saturating_sub(3)].join("\t"))
    };
    checks.require(
        construction_case_map
            .values()
            .next()
            .is_some_and(|line| truncate_columns(line).split('\t').count() != 11),
        "schema-construction fixture column truncation control did not shorten a case",
    );
    let toggle_polarity = |line: &str| -> String {
        let mut fields = line.trim_end().split('\t').collect::<Vec<_>>();
        fields[2] = "negative";
        format!("{}\n", fields.join("\t"))
    };
    checks.require(
        require_construction_case(
            &{
                let mut cases = construction_case_map.clone();
                let id = cases
                    .keys()
                    .next()
                    .expect("construction fixture ID")
                    .clone();
                cases.insert(id.clone(), truncate_columns(&cases[&id]));
                cases
            },
            construction_case_map
                .keys()
                .next()
                .expect("construction fixture ID"),
            "positive",
            "accept",
            rows.first().expect("inventory row"),
        )
        .is_err(),
        "require_construction_case accepted a truncated fixture row",
    );
    checks.require(
        require_construction_case(
            &{
                let mut cases = construction_case_map.clone();
                let id = cases
                    .keys()
                    .next()
                    .expect("construction fixture ID")
                    .clone();
                cases.insert(id.clone(), toggle_polarity(&cases[&id]));
                cases
            },
            construction_case_map
                .keys()
                .next()
                .expect("construction fixture ID"),
            "positive",
            "accept",
            rows.first().expect("inventory row"),
        )
        .is_err(),
        "require_construction_case accepted a fixture row with the wrong polarity",
    );
    // Classify every field row in a single pass (rather than one `.find()`
    // per predicate) so that rows before each target are still visited and
    // both sides of the `dependent`/`repository artifact` and
    // `is_state_dependent_collection` conditions are genuinely exercised.
    let mut omitted_row: Option<&Row> = None;
    let mut plain_option_row: Option<&Row> = None;
    let mut dependent_option_row: Option<&Row> = None;
    let mut state_dependent_collection_row: Option<&Row> = None;
    let mut plain_collection_row: Option<&Row> = None;
    for row in rows.iter().filter(|row| row.category == "field") {
        if omitted_row.is_none() {
            omitted_row = Some(row);
        }
        if is_option_field(row) {
            let dependent =
                row.rule.contains("dependent") || row.rule.contains("repository artifact");
            if dependent {
                dependent_option_row.get_or_insert(row);
            } else {
                plain_option_row.get_or_insert(row);
            }
        }
        if is_collection_field(row) {
            if is_state_dependent_collection(row) {
                state_dependent_collection_row.get_or_insert(row);
            } else {
                plain_collection_row.get_or_insert(row);
            }
        }
    }
    let construction_targets: Vec<(&str, String)> = vec![
        (
            "omitted",
            omitted_row
                .map(|row| format!("field-{}-{}-omitted", row.parent, row.name))
                .expect("field row"),
        ),
        (
            "explicit-none/some",
            plain_option_row
                .map(|row| format!("field-{}-{}-explicit-none", row.parent, row.name))
                .expect("plain option field row"),
        ),
        (
            "invalid-option-shape",
            dependent_option_row
                .map(|row| format!("field-{}-{}-invalid-option-shape", row.parent, row.name))
                .expect("dependent option field row"),
        ),
        (
            "explicit-empty-allowed-state",
            state_dependent_collection_row
                .map(|row| {
                    format!(
                        "field-{}-{}-explicit-empty-allowed-state",
                        row.parent, row.name
                    )
                })
                .expect("state-dependent collection field row"),
        ),
        (
            "explicit-empty-required-state",
            state_dependent_collection_row
                .map(|row| {
                    format!(
                        "field-{}-{}-explicit-empty-required-state",
                        row.parent, row.name
                    )
                })
                .expect("state-dependent collection field row"),
        ),
        (
            "explicit-empty",
            plain_collection_row
                .map(|row| format!("field-{}-{}-explicit-empty", row.parent, row.name))
                .expect("plain collection field row"),
        ),
    ];
    for (label, id) in construction_targets {
        checks.require(
            construction_case_map.contains_key(&id),
            &format!("schema-construction fixture coverage target `{label}` was not generated"),
        );
        let mut corrupted = construction_case_map.clone();
        corrupted.remove(&id);
        checks.require(
            audit_construction_fixture_coverage(&rows, &corrupted).is_err(),
            &format!("schema-construction fixture coverage accepted a missing `{label}` case"),
        );
    }
    for category in ["enum-variant", "enum", "reference-kind", "object"] {
        let target_row = rows
            .iter()
            .find(|row| row.category == category)
            .expect("row of the given category");
        let id = match category {
            "enum-variant" => format!("enum-{}-{}-known", target_row.parent, target_row.name),
            "enum" => format!("enum-{}-unknown", target_row.name),
            "reference-kind" => format!("reference-{}-current-valid", target_row.name),
            _ => format!("object-{}-metadata-kind-exact", target_row.name),
        };
        let mut corrupted = construction_case_map.clone();
        corrupted.remove(&id);
        checks.require(
            audit_construction_fixture_coverage(&rows, &corrupted).is_err(),
            &format!("schema-construction fixture coverage accepted a missing `{category}` case"),
        );
    }
    let reference_row = rows
        .iter()
        .find(|row| row.category == "reference-kind")
        .expect("reference-kind row");
    let reference_invalid_id = format!("reference-{}-invalid-binding", reference_row.name);
    construction_case_map.remove(&reference_invalid_id);
    checks.require(
        audit_construction_fixture_coverage(&rows, &construction_case_map).is_err(),
        "schema-construction fixture coverage accepted a missing reference-kind invalid-binding case",
    );
    let mut construction_case_map = expected_fixtures
        .lines()
        .skip(1)
        .map(|line| {
            let id = line
                .split('\t')
                .nth(1)
                .expect("construction fixture case ID field")
                .to_owned();
            (id, format!("{line}\n"))
        })
        .collect::<BTreeMap<_, _>>();
    let object_row = rows
        .iter()
        .find(|row| row.category == "object")
        .expect("object row");
    let object_mismatch_id = format!("object-{}-metadata-kind-mismatch", object_row.name);
    construction_case_map.remove(&object_mismatch_id);
    checks.require(
        audit_construction_fixture_coverage(&rows, &construction_case_map).is_err(),
        "schema-construction fixture coverage accepted a missing object metadata-kind-mismatch case",
    );
    let mut construction_case_map = expected_fixtures
        .lines()
        .skip(1)
        .map(|line| {
            let id = line
                .split('\t')
                .nth(1)
                .expect("construction fixture case ID field")
                .to_owned();
            (id, format!("{line}\n"))
        })
        .collect::<BTreeMap<_, _>>();
    let terminal_row = rows
        .iter()
        .find(|row| is_terminal_transition(row))
        .expect("terminal-transition row");
    let terminal_complete_id = format!(
        "terminal-{}-{}-complete",
        terminal_row.parent, terminal_row.name
    );
    let terminal_incomplete_id = format!(
        "terminal-{}-{}-incomplete",
        terminal_row.parent, terminal_row.name
    );
    construction_case_map.remove(&terminal_complete_id);
    checks.require(
        audit_construction_fixture_coverage(&rows, &construction_case_map).is_err(),
        "schema-construction fixture coverage accepted a missing terminal-complete case",
    );
    let mut construction_case_map = expected_fixtures
        .lines()
        .skip(1)
        .map(|line| {
            let id = line
                .split('\t')
                .nth(1)
                .expect("construction fixture case ID field")
                .to_owned();
            (id, format!("{line}\n"))
        })
        .collect::<BTreeMap<_, _>>();
    construction_case_map.remove(&terminal_incomplete_id);
    checks.require(
        audit_construction_fixture_coverage(&rows, &construction_case_map).is_err(),
        "schema-construction fixture coverage accepted a missing terminal-incomplete case",
    );

    // `check_declared_parents` impact-edge negative controls.
    let impact_edge_row = rows
        .iter()
        .find(|row| row.category == "impact-edge")
        .expect("impact-edge row")
        .clone();
    let mut declarations_for_edges = BTreeMap::<String, BTreeSet<String>>::new();
    for row in &rows {
        if matches!(
            row.category.as_str(),
            "domain" | "nested" | "report-fact" | "enum"
        ) {
            declarations_for_edges
                .entry(row.category.clone())
                .or_default()
                .insert(row.name.clone());
        }
    }
    let mut undeclared_source = impact_edge_row.clone();
    undeclared_source.parent = String::from("self-test-undeclared-source");
    checks.require(
        check_declared_parents(&[undeclared_source], &declarations_for_edges).is_err(),
        "check_declared_parents accepted an undeclared impact-edge source",
    );
    let mut undeclared_target = impact_edge_row.clone();
    undeclared_target.shape = String::from("target:self-test-undeclared-target");
    checks.require(
        check_declared_parents(&[undeclared_target], &declarations_for_edges).is_err(),
        "check_declared_parents accepted an undeclared impact-edge target",
    );
    let mut undeclared_source_field = impact_edge_row.clone();
    undeclared_source_field.rule = String::from("source-field:self-test-undeclared-field");
    checks.require(
        check_declared_parents(&[undeclared_source_field], &declarations_for_edges).is_err(),
        "check_declared_parents accepted an undeclared impact-edge source field",
    );

    // `require_impact_case` and `insert_impact_case` negative controls.
    let mut impact_case_map = BTreeMap::new();
    insert_impact_case(
        &mut impact_case_map,
        String::from("self-test-impact-control"),
        "positive",
        "edge",
        "source",
        "target",
        "target",
        1,
        "none",
        "accept",
        "-",
        "target",
    )
    .expect("initial impact-edge fixture ID control was accepted");
    checks.require(
        insert_impact_case(
            &mut impact_case_map,
            String::from("self-test-impact-control"),
            "negative",
            "edge",
            "source",
            "target",
            "target",
            1,
            "mutation",
            "reject",
            "CHG-CONTROL",
            "target",
        )
        .is_err(),
        "insert_impact_case accepted a duplicate impact-edge fixture ID",
    );
    checks.require(
        require_impact_case(
            &impact_case_map,
            "self-test-missing-impact-case",
            "positive",
            "accept",
        )
        .is_err(),
        "require_impact_case accepted a missing impact-edge fixture ID",
    );
    let mut malformed_impact_case_map = impact_case_map.clone();
    malformed_impact_case_map.insert(
        String::from("self-test-impact-control"),
        String::from("malformed\n"),
    );
    checks.require(
        require_impact_case(
            &malformed_impact_case_map,
            "self-test-impact-control",
            "positive",
            "accept",
        )
        .is_err(),
        "require_impact_case accepted a malformed impact-edge fixture row",
    );
    checks.require(
        require_impact_case(&impact_case_map, "self-test-impact-control", "negative", "accept")
            .is_err(),
        "require_impact_case accepted an impact-edge fixture row with the wrong polarity or outcome",
    );

    // `validate_state_fixture_source` / `validate_policy_fixture_source` /
    // `validate_disposition_fixture_source` structural and closing-check
    // negative controls not already covered above.
    for control in [
        state_source.trim_end_matches('\n').to_owned(),
        format!("wrong-header\n{state_first_case}\n"),
        format!("{STATE_FIXTURE_HEADER}\nmalformed\n"),
    ] {
        checks.require(
            validate_state_fixture_source(&control).is_err(),
            "malformed schema-state structural negative control was accepted",
        );
    }
    let state_target_subject = REQUIRED_STATE_SUBJECTS[0];
    let subject_incomplete_state = state_source
        .lines()
        .map(|line| {
            let mut fields = line.split('\t').collect::<Vec<_>>();
            if fields[2] == "negative" && fields[3] == state_target_subject {
                fields[2] = "positive";
                fields[8] = "accept";
                fields[9] = "-";
            }
            fields.join("\t")
        })
        .collect::<Vec<_>>()
        .join("\n")
        + "\n";
    checks.require(
        validate_state_fixture_source(&subject_incomplete_state).is_err(),
        "schema-state subject-incomplete negative control was accepted",
    );
    checks.require(
        validate_state_fixture_source(&state_source.replacen(
            state_first_case,
            &format!("{state_first_case}\n{state_first_case}"),
            1,
        ))
        .is_err(),
        "schema-state duplicate-case-ID negative control was accepted",
    );
    let state_negative_case = state_source
        .lines()
        .find(|line| line.contains("\tnegative\t"))
        .expect("negative schema-state case");
    for control in [
        state_source.replacen(
            state_negative_case,
            &state_negative_case.replacen("\treject\tSCF-", "\taccept\tSCF-", 1),
            1,
        ),
        state_source.replacen(
            state_negative_case,
            &state_negative_case.replacen("\tSCF-", "\tWRONG-", 1),
            1,
        ),
    ] {
        checks.require(
            validate_state_fixture_source(&control).is_err(),
            "malformed schema-state negative-case negative control was accepted",
        );
    }

    for control in [
        policy_source.trim_end_matches('\n').to_owned(),
        format!("wrong-header\n{policy_first_case}\n"),
        format!("{POLICY_FIXTURE_HEADER}\nmalformed\n"),
    ] {
        checks.require(
            validate_policy_fixture_source(&control).is_err(),
            "malformed policy-ratchet structural negative control was accepted",
        );
    }
    checks.require(
        validate_policy_fixture_source(&policy_source.replacen(
            policy_first_case,
            &format!("{policy_first_case}\n{policy_first_case}"),
            1,
        ))
        .is_err(),
        "policy-ratchet duplicate-case-ID negative control was accepted",
    );
    let policy_target_dimension = REQUIRED_POLICY_DIMENSIONS[0];
    let dimension_incomplete_policy = policy_source
        .lines()
        .map(|line| {
            let mut fields = line.split('\t').collect::<Vec<_>>();
            if fields[2] == "negative" && fields[3] == policy_target_dimension {
                fields[2] = "positive";
                fields[7] = "strengthening";
                fields[8] = "accept";
                fields[9] = "-";
            }
            fields.join("\t")
        })
        .collect::<Vec<_>>()
        .join("\n")
        + "\n";
    checks.require(
        validate_policy_fixture_source(&dimension_incomplete_policy).is_err(),
        "policy-ratchet dimension-incomplete negative control was accepted",
    );
    checks.require(
        validate_policy_fixture_source(&policy_source.replacen("\tauthority\t", "\t\t", 1))
            .is_err(),
        "policy-ratchet empty-field negative control was accepted",
    );
    let policy_negative_case = policy_source
        .lines()
        .find(|line| line.contains("\tnegative\t"))
        .expect("negative policy-ratchet case");
    checks.require(
        validate_policy_fixture_source(&policy_source.replacen(
            policy_negative_case,
            &policy_negative_case.replacen("CHG-RATCHET-", "WRONG-RATCHET-", 1),
            1,
        ))
        .is_err(),
        "policy-ratchet mismatched-diagnostic negative control was accepted",
    );

    for control in [
        disposition_source.trim_end_matches('\n').to_owned(),
        format!("wrong-header\n{disposition_first_case}\n"),
        format!("{DISPOSITION_FIXTURE_HEADER}\nmalformed\n"),
    ] {
        checks.require(
            validate_disposition_fixture_source(&control).is_err(),
            "malformed disposition-protocol structural negative control was accepted",
        );
    }
    checks.require(
        validate_disposition_fixture_source(&disposition_source.replacen(
            disposition_first_case,
            &format!("{disposition_first_case}\n{disposition_first_case}"),
            1,
        ))
        .is_err(),
        "disposition-protocol duplicate-case-ID negative control was accepted",
    );
    let disposition_ids_mismatch = format!(
        "{disposition_source}{DISPOSITION_FIXTURE_SCHEMA}\tzzz-self-test-extra\tpositive\t{}\tnone\t$graph:base\t$graph:head\taccept\t-\tno-facts\n",
        REQUIRED_DISPOSITION_KINDS[0]
    );
    checks.require(
        validate_disposition_fixture_source(&disposition_ids_mismatch).is_err(),
        "disposition-protocol ID-set mismatch negative control was accepted",
    );
    for control in [
        disposition_source.replacen("\tpositive\treaffirmed\t", "\tpositive\tretired\t", 1),
        disposition_source.replacen(
            "\tnegative\treaffirmed\tapplicability\t",
            "\tnegative\tretired\tapplicability\t",
            1,
        ),
        disposition_source.replacen("CHG-DISPOSITION-", "WRONG-DIAGNOSTIC-", 1),
        disposition_source.replacen("\treaffirmed\t", "\t\t", 1),
    ] {
        checks.require(
            validate_disposition_fixture_source(&control).is_err(),
            "malformed disposition-protocol kind/mutation negative control was accepted",
        );
    }

    // `validate_canonical_vector_parity` structural negative controls.
    let canonical_first_vector = canonical_vectors
        .lines()
        .nth(1)
        .expect("validated canonical vector matrix has cases");
    for control in [
        canonical_vectors.trim_end_matches('\n').to_owned(),
        canonical_vectors.replacen(CANONICAL_VECTOR_HEADER, "wrong-header", 1),
        format!("{CANONICAL_VECTOR_HEADER}\nmalformed\n"),
        canonical_vectors.replacen(
            canonical_first_vector,
            &format!("{canonical_first_vector}\n{canonical_first_vector}"),
            1,
        ),
        canonical_vectors.replacen(
            "spec-provenance-canonical-vector/v2\t",
            "wrong-vector-schema\t",
            1,
        ),
    ] {
        checks.require(
            validate_canonical_vector_parity(&rows, &control).is_err(),
            "malformed canonical-vector negative control was accepted",
        );
    }

    // `audit_enum_variant_tags` / `audit_object_kind_tags` /
    // `audit_validators` / `audit_api_bindings` / `braced_block_from` /
    // `pascal_case` negative controls not already covered above.
    checks.require(
        braced_block_from("marker }", "marker").is_none(),
        "braced_block_from accepted a marker with no opening brace",
    );
    checks.require(
        braced_block_from("marker {", "marker").is_none(),
        "braced_block_from accepted a marker whose brace never closes",
    );
    checks.require(
        pascal_case("").is_empty(),
        "pascal_case did not map an empty value to empty",
    );
    checks.require(
        pascal_case("--value") == "Value",
        "pascal_case did not title-case a hyphenated value",
    );
    checks.require(
        declared_rust_variants("bad\n = 1\nBad-name = 1\nValid = 1\nInvalid = nope\n")
            == BTreeSet::from([String::from("Valid")]),
        "declared_rust_variants accepted an empty name, a non-identifier name, or a non-digit value",
    );
    checks.require(
        has_validator("snapshot-budgets", "fn validate(", ""),
        "has_validator rejected a snapshot-budgets block with a validate fn",
    );
    checks.require(
        !has_validator("snapshot-budgets", "", ""),
        "has_validator accepted a snapshot-budgets block with no validate fn",
    );
    checks.require(
        has_validator(
            "oracle-command",
            "",
            "for oracle in &config.oracles #[serde(deny_unknown_fields)]",
        ),
        "has_validator rejected an oracle-command implementation with both required anchors",
    );
    checks.require(
        !has_validator("oracle-command", "", ""),
        "has_validator accepted an oracle-command implementation with neither required anchor",
    );
    checks.require(
        !has_validator("oracle-command", "", "for oracle in &config.oracles"),
        "has_validator accepted an oracle-command implementation missing its second required anchor",
    );
    for name in ["source-child-reference", "source-edge", "source-budgets"] {
        checks.require(
            has_validator(name, "pub const fn new(", ""),
            "has_validator rejected a source value const constructor",
        );
        checks.require(
            !has_validator(name, "", ""),
            "has_validator accepted a source value without a const constructor",
        );
    }
    checks.require(
        has_validator("control", "pub fn new(", ""),
        "has_validator rejected a generic block with a pub fn new",
    );
    checks.require(
        !has_validator("control", "", ""),
        "has_validator accepted a generic block with no pub fn new",
    );

    let mut unknown_enum_parent_rows = rows.clone();
    unknown_enum_parent_rows
        .iter_mut()
        .find(|row| row.category == "enum-variant")
        .expect("enum-variant row")
        .parent = String::from("self-test-unknown-enum-parent");
    checks.require(
        audit_enum_variant_tags(&unknown_enum_parent_rows, "").is_err(),
        "audit_enum_variant_tags accepted an unmapped enum parent",
    );
    checks.require(
        audit_enum_variant_tags(&rows, "").is_err(),
        "audit_enum_variant_tags accepted an implementation missing every declaration",
    );
    let mut untagged_enum_variant_rows = rows.clone();
    untagged_enum_variant_rows
        .iter_mut()
        .find(|row| row.category == "enum-variant")
        .expect("enum-variant row")
        .tag = None;
    checks.require(
        audit_enum_variant_tags(&untagged_enum_variant_rows, "pub enum DigestAlgorithm { }")
            .is_err(),
        "audit_enum_variant_tags accepted an untagged enum variant",
    );
    let one_enum_row = rows
        .iter()
        .find(|row| row.category == "enum-variant")
        .expect("enum-variant row")
        .clone();
    let enum_anchor = ENUM_API_BINDINGS
        .iter()
        .find(|(name, _)| *name == one_enum_row.parent)
        .expect("enum binding")
        .1;
    let enum_with_extra_variant = format!(
        "{enum_anchor} {{ {} = {}, Extra = 65535 }}",
        pascal_case(&one_enum_row.name),
        one_enum_row.tag.expect("enum tag")
    );
    checks.require(
        audit_enum_variant_tags(&[one_enum_row], &enum_with_extra_variant).is_err(),
        "audit_enum_variant_tags accepted an implementation declaring an extra variant",
    );

    let mut untagged_object_rows = rows.clone();
    untagged_object_rows
        .iter_mut()
        .find(|row| row.category == "object")
        .expect("object row")
        .tag = None;
    checks.require(
        audit_object_kind_tags(&untagged_object_rows, "pub enum ObjectKind { }").is_err(),
        "audit_object_kind_tags accepted an untagged object",
    );
    checks.require(
        audit_object_kind_tags(&rows, "").is_err(),
        "audit_object_kind_tags accepted an implementation missing the ObjectKind declaration",
    );
    let one_object_row = rows
        .iter()
        .find(|row| row.category == "object")
        .expect("object row")
        .clone();
    let object_kind_tag_mismatch = format!(
        "pub enum ObjectKind {{ {} = 65535 }}",
        pascal_case(&one_object_row.name)
    );
    checks.require(
        audit_object_kind_tags(&[one_object_row], &object_kind_tag_mismatch).is_err(),
        "audit_object_kind_tags accepted a mismatched object tag",
    );
    let object_kind_with_extra_variant = rows.iter().filter(|row| row.category == "object").fold(
        String::from("pub enum ObjectKind { Extra = 65535,"),
        |mut text, row| {
            let _ = write!(
                text,
                "{} = {},",
                pascal_case(&row.name),
                row.tag.expect("object tag")
            );
            text
        },
    ) + "}";
    checks.require(
        audit_object_kind_tags(&rows, &object_kind_with_extra_variant).is_err(),
        "audit_object_kind_tags accepted an implementation declaring an extra ObjectKind variant",
    );

    checks.require(
        audit_validators(
            "control",
            &[("control", "pub struct Control")],
            "impl Control { pub fn new( }",
        )
        .is_ok(),
        "audit_validators rejected a valid construction anchor",
    );
    checks.require(
        audit_validators("control", &[("control", "")], "").is_err(),
        "audit_validators accepted an empty type anchor",
    );
    checks.require(
        audit_validators("control", &[("control", "pub struct Missing")], "").is_err(),
        "audit_validators accepted a missing implementation block",
    );
    checks.require(
        audit_validators(
            "control",
            &[("control", "pub struct Control")],
            "impl Control { }",
        )
        .is_err(),
        "audit_validators accepted a block with no validated construction anchor",
    );
    for implementation in [
        "",
        "SourceItemKind,\n    versioned_lowercase_id_grammar",
        "SourceItemKind,\n    versioned_lowercase_id_grammar\nmacro_rules! identifier_newtype",
    ] {
        checks.require(
            audit_validators(
                "control",
                &[("source-item-kind", "SourceItemKind,")],
                implementation,
            )
            .is_err(),
            "audit_validators accepted a missing source-item-kind construction anchor",
        );
    }
    for result in [
        audit_validators("object", &OBJECT_API_BINDINGS, ""),
        audit_validators("nested", &NESTED_API_BINDINGS, ""),
        audit_validators("report-fact", &REPORT_API_BINDINGS, ""),
    ] {
        checks.require(
            result.is_err(),
            "audit_validators accepted an empty implementation",
        );
    }
    for result in [
        audit_api_bindings(&[], "enum", &ENUM_API_BINDINGS, ""),
        audit_api_bindings(&[], "object", &OBJECT_API_BINDINGS, ""),
        audit_api_bindings(&[], "nested", &NESTED_API_BINDINGS, ""),
        audit_api_bindings(&[], "report-fact", &REPORT_API_BINDINGS, ""),
        audit_api_bindings(&[], "domain", &DOMAIN_API_BINDINGS, ""),
    ] {
        checks.require(
            result.is_err(),
            "audit_api_bindings accepted an empty row set",
        );
    }

    // `validate_api_parity_sources` negative controls not already covered
    // by the schema API parity block above.
    for category in ["enum", "object", "nested", "report-fact", "domain"] {
        let mut removed = false;
        let rows_without_category_member = rows
            .iter()
            .filter(|row| {
                if !removed && row.category == category {
                    removed = true;
                    false
                } else {
                    true
                }
            })
            .cloned()
            .collect::<Vec<_>>();
        checks.require(
            validate_api_parity_sources(
                &rows_without_category_member,
                &implementation,
                &canonical_vectors,
            )
            .is_err(),
            &format!("schema API parity accepted a missing `{category}` inventory member"),
        );
    }
    let object_row_for_tag = rows
        .iter()
        .find(|row| row.category == "object")
        .expect("object row");
    let object_tag_pattern = format!(
        "{} = {},",
        pascal_case(&object_row_for_tag.name),
        object_row_for_tag.tag.expect("object tag")
    );
    let object_kind_declaration_block = braced_block_from(&implementation, "pub enum ObjectKind")
        .expect("public ObjectKind declaration is present")
        .to_owned();
    checks.require(
        object_kind_declaration_block.contains(&object_tag_pattern),
        "ObjectKind declaration does not contain the expected object tag pattern",
    );
    let corrupted_object_kind_block = object_kind_declaration_block.replacen(
        &object_tag_pattern,
        &format!("{} = 65535,", pascal_case(&object_row_for_tag.name)),
        1,
    );
    let object_tag_mismatch_implementation = implementation.replacen(
        &object_kind_declaration_block,
        &corrupted_object_kind_block,
        1,
    );
    // `object-kind` is also registered as a mirrored `enum`/`enum-variant`
    // binding onto the same `pub enum ObjectKind` declaration, so
    // `audit_enum_variant_tags` (called before `audit_object_kind_tags`
    // inside `validate_api_parity_sources`) would otherwise reject this
    // corrupted declaration first. Drop its mirrored enum-variant rows so
    // this control isolates `audit_object_kind_tags`'s own per-row check.
    let rows_without_object_kind_enum_variants = rows
        .iter()
        .filter(|row| !(row.category == "enum-variant" && row.parent == "object-kind"))
        .cloned()
        .collect::<Vec<_>>();
    checks.require(
        validate_api_parity_sources(
            &rows_without_object_kind_enum_variants,
            &object_tag_mismatch_implementation,
            &canonical_vectors,
        )
        .is_err(),
        "schema API parity accepted a mismatched ObjectKind tag",
    );
    let missing_nested_validator =
        implementation.replacen("impl ObjectRef {", "impl ObjectRefWithoutValidator {", 1);
    checks.require(
        validate_api_parity_sources(&rows, &missing_nested_validator, &canonical_vectors).is_err(),
        "schema API parity accepted a nested type missing its validator",
    );
    let missing_report_validator = implementation.replacen(
        "impl AuthorityFact {",
        "impl AuthorityFactWithoutValidator {",
        1,
    );
    checks.require(
        validate_api_parity_sources(&rows, &missing_report_validator, &canonical_vectors).is_err(),
        "schema API parity accepted a report-fact type missing its validator",
    );
    let missing_domain_block = implementation.replacen(
        "impl RepositoryConfig {",
        "impl RepositoryConfig /* self-test missing block */ {",
        1,
    );
    checks.require(
        validate_api_parity_sources(&rows, &missing_domain_block, &canonical_vectors).is_err(),
        "schema API parity accepted a missing domain encoder block",
    );
    let missing_canonical_vector =
        canonical_vectors.replacen(&format!("{canonical_first_vector}\n"), "", 1);
    checks.require(
        validate_api_parity_sources(&rows, &implementation, &missing_canonical_vector).is_err(),
        "schema API parity accepted a canonical-vector matrix missing a required row",
    );

    checks.finish()
}

#[cfg(all(test, feature = "coverage-controls"))]
#[allow(clippy::expect_used, clippy::format_push_string)]
mod coverage_tests {
    use super::*;

    #[test]
    fn test_binary_variant_runs_every_command_path() {
        run_with_arguments(&[]).expect("default schema inventory command");
        run_with_arguments(&[String::from("--self-test")]).expect("schema inventory self-test");
        run_with_arguments(&[String::from(DEFAULT_INVENTORY)])
            .expect("explicit schema inventory command");
        let temporary_inventory = tempfile::NamedTempFile::new().expect("temporary inventory");
        fs::write(
            temporary_inventory.path(),
            read_text(DEFAULT_INVENTORY).expect("inventory source"),
        )
        .expect("write temporary inventory");
        let temporary_path = temporary_inventory.path().to_string_lossy().into_owned();
        run_with_arguments(std::slice::from_ref(&temporary_path))
            .expect("non-default schema inventory command");
        fs::write(temporary_inventory.path(), "malformed\n")
            .expect("write malformed temporary inventory");
        assert!(run_with_arguments(&[temporary_path]).is_err());
        FORCED_FIXTURE_FAILURE.store(1, Ordering::Relaxed);
        assert!(run_with_arguments(&[]).is_err());
        FORCED_FIXTURE_FAILURE.store(2, Ordering::Relaxed);
        assert!(run_with_arguments(&[]).is_err());
        FORCED_FIXTURE_FAILURE.store(3, Ordering::Relaxed);
        assert!(run_with_arguments(&[]).is_err());
        FORCED_FIXTURE_FAILURE.store(4, Ordering::Relaxed);
        assert!(run_with_arguments(&[]).is_err());
        FORCED_FIXTURE_FAILURE.store(5, Ordering::Relaxed);
        assert!(run_with_arguments(&[]).is_err());
        FORCED_FIXTURE_FAILURE.store(6, Ordering::Relaxed);
        assert!(run_with_arguments(&[]).is_err());
        assert!(run_with_arguments(&[String::from("missing-inventory.tsv")]).is_err());
        assert!(write_text(".", "not-written").is_err());
        assert!(run_with_arguments(&[String::from("one"), String::from("two")]).is_err());
    }

    fn inventory_rows() -> Vec<Row> {
        read_text(DEFAULT_INVENTORY)
            .expect("inventory source")
            .lines()
            .skip(1)
            .enumerate()
            .map(|(index, line)| parse_row(index + 2, line).expect("validated inventory row"))
            .collect()
    }

    fn fixture_cases(source: &str) -> BTreeMap<String, String> {
        source
            .lines()
            .skip(1)
            .map(|line| {
                let id = line.split('\t').nth(1).expect("fixture case ID").to_owned();
                (id, format!("{line}\n"))
            })
            .collect()
    }

    #[test]
    fn test_schema_inventory_negative_path_controls() {
        let rows = inventory_rows();
        let source = read_text(DEFAULT_INVENTORY).expect("inventory source");

        for control in [
            source.replacen("target:atom-object", "target:missing-object", 1),
            source.replacen("source-field:replacements", "source-field:missing-field", 1),
            source.replacen(
                "impact-edge\tformal-property",
                "impact-edge\tmissing-parent",
                1,
            ),
        ] {
            assert!(validate(&control).is_err());
        }

        let (fixture_source, _) = render_construction_fixtures(&rows);
        let cases = fixture_cases(&fixture_source);
        for id in cases.keys() {
            let mut incomplete = cases.clone();
            incomplete.remove(id);
            assert!(audit_construction_fixture_coverage(&rows, &incomplete).is_err());
        }
        let mut malformed_cases = cases.clone();
        let malformed_id = malformed_cases.keys().next().expect("fixture ID").clone();
        malformed_cases.insert(malformed_id.clone(), String::from("malformed\n"));
        assert!(
            require_construction_case(
                &malformed_cases,
                &malformed_id,
                "positive",
                "accept",
                rows.first().expect("inventory row"),
            )
            .is_err()
        );
        let mut wrong_case = cases.clone();
        let first_id = wrong_case.keys().next().expect("fixture ID").clone();
        wrong_case.insert(
            first_id.clone(),
            wrong_case[&first_id].replacen("\tpositive\t", "\tnegative\t", 1),
        );
        assert!(
            require_construction_case(
                &wrong_case,
                &first_id,
                "positive",
                "accept",
                rows.first().expect("inventory row"),
            )
            .is_err()
        );

        let state = read_text(DEFAULT_STATE_FIXTURES).expect("state fixtures");
        let state_first = state.lines().nth(1).expect("state case");
        let state_negative = state
            .lines()
            .find(|line| line.contains("\tnegative\t"))
            .expect("negative state case");
        for control in [
            state.trim_end_matches('\n').to_owned(),
            format!("wrong-header\n{state_first}\n"),
            format!("{STATE_FIXTURE_HEADER}\nmalformed\n"),
            state.replacen(state_first, &format!("{state_first}\n{state_first}"), 1),
            state.replacen("approval-exact-current", "approval-aaa-current", 1),
            state.replacen(
                "\tpositive\tapproval-binding\t",
                "\tpositive\tartifact-index\t",
                1,
            ),
            state.replacen(
                state_negative,
                &state_negative.replacen("\treject\tSCF-", "\taccept\tSCF-", 1),
                1,
            ),
            state.replacen(
                state_negative,
                &state_negative.replacen("\tSCF-", "\tWRONG-", 1),
                1,
            ),
            state.replacen(
                state_negative,
                &state_negative.replacen(
                    state_negative.split('\t').nth(9).expect("state diagnostic"),
                    "-",
                    1,
                ),
                1,
            ),
        ] {
            assert!(validate_state_fixture_source(&control).is_err());
        }

        let policy = read_text(DEFAULT_POLICY_FIXTURES).expect("policy fixtures");
        let policy_first = policy.lines().nth(1).expect("policy case");
        let policy_negative = policy
            .lines()
            .find(|line| line.contains("\tnegative\t"))
            .expect("negative policy case");
        for control in [
            policy.trim_end_matches('\n').to_owned(),
            format!("wrong-header\n{policy_first}\n"),
            format!("{POLICY_FIXTURE_HEADER}\nmalformed\n"),
            policy.replacen(policy_first, &format!("{policy_first}\n{policy_first}"), 1),
            policy.replacen(
                "policy-authority-hidden-weakening",
                "policy-authority-aaa-weakening",
                1,
            ),
            policy.replacen("\tpositive\tauthority\t", "\tpositive\tderivation\t", 1),
            policy.replacen(
                policy_negative,
                &policy_negative.replacen("CHG-RATCHET-", "WRONG-RATCHET-", 1),
                1,
            ),
            policy.replacen("\tauthority\t", "\t\t", 1),
        ] {
            assert!(validate_policy_fixture_source(&control).is_err());
        }

        let disposition = read_text(DEFAULT_DISPOSITION_FIXTURES).expect("disposition fixtures");
        let disposition_first = disposition.lines().nth(1).expect("disposition case");
        for control in [
            disposition.trim_end_matches('\n').to_owned(),
            format!("wrong-header\n{disposition_first}\n"),
            format!("{DISPOSITION_FIXTURE_HEADER}\nmalformed\n"),
            disposition.replacen(
                disposition_first,
                &format!("{disposition_first}\n{disposition_first}"),
                1,
            ),
            disposition.replacen(
                "disposition-reaffirmed-complete",
                "disposition-reaffirmed-aaa-complete",
                1,
            ),
            disposition.replacen("\tpositive\treaffirmed\t", "\tpositive\tretired\t", 1),
            disposition.replacen(
                "\tnegative\treaffirmed\tapplicability\t",
                "\tnegative\tretired\tapplicability\t",
                1,
            ),
            disposition.replacen("CHG-DISPOSITION-", "WRONG-DIAGNOSTIC-", 1),
            disposition.replacen("\treaffirmed\t", "\t\t", 1),
        ] {
            assert!(validate_disposition_fixture_source(&control).is_err());
        }

        assert!(braced_block_from("marker }", "marker").is_none());
        assert!(braced_block_from("marker {", "marker").is_none());
        assert_eq!(pascal_case(""), "");
        assert_eq!(pascal_case("--value"), "Value");
        assert!(
            declared_rust_variants("bad\n = 1\nBad-name = 1\nValid = 1\nInvalid = nope\n")
                .contains("Valid")
        );

        let mut invalid_rows = rows.clone();
        let enum_row = invalid_rows
            .iter_mut()
            .find(|row| row.category == "enum-variant")
            .expect("enum row");
        enum_row.parent = String::from("missing-enum");
        assert!(audit_enum_variant_tags(&invalid_rows, "").is_err());
        assert!(audit_enum_variant_tags(&rows, "").is_err());

        let one_enum_row = rows
            .iter()
            .find(|row| row.category == "enum-variant")
            .expect("enum row")
            .clone();
        let enum_anchor = ENUM_API_BINDINGS
            .iter()
            .find(|(name, _)| *name == one_enum_row.parent)
            .expect("enum binding")
            .1;
        let enum_with_extra = format!(
            "{enum_anchor} {{ {} = {}, Extra = 65535 }}",
            pascal_case(&one_enum_row.name),
            one_enum_row.tag.expect("enum tag")
        );
        assert!(audit_enum_variant_tags(&[one_enum_row], &enum_with_extra).is_err());

        let mut invalid_rows = rows.clone();
        let enum_row = invalid_rows
            .iter_mut()
            .find(|row| row.category == "enum-variant")
            .expect("enum row");
        enum_row.tag = None;
        assert!(audit_enum_variant_tags(&invalid_rows, "pub enum DigestAlgorithm { }").is_err());

        let mut invalid_rows = rows.clone();
        invalid_rows
            .iter_mut()
            .find(|row| row.category == "object")
            .expect("object row")
            .tag = None;
        assert!(audit_object_kind_tags(&invalid_rows, "pub enum ObjectKind { }").is_err());
        assert!(audit_object_kind_tags(&rows, "").is_err());
        assert!(audit_object_kind_tags(&rows, "pub enum ObjectKind { }").is_err());
        let object_declaration = rows.iter().filter(|row| row.category == "object").fold(
            String::from("pub enum ObjectKind { Extra = 65535,"),
            |mut text, row| {
                text.push_str(&format!(
                    "{} = {},",
                    pascal_case(&row.name),
                    row.tag.expect("object tag")
                ));
                text
            },
        ) + "}";
        assert!(audit_object_kind_tags(&rows, &object_declaration).is_err());

        assert!(
            audit_validators(
                "control",
                &[("control", "pub struct Control")],
                "impl Control { pub fn new( }"
            )
            .is_ok()
        );
        assert!(audit_validators("control", &[("control", "")], "").is_err());
        assert!(audit_validators("control", &[("control", "pub struct Missing")], "").is_err());
        assert!(
            audit_validators(
                "control",
                &[("control", "pub struct Control")],
                "impl Control { }"
            )
            .is_err()
        );
        for result in [
            audit_validators("object", &OBJECT_API_BINDINGS, ""),
            audit_validators("nested", &NESTED_API_BINDINGS, ""),
            audit_validators("report-fact", &REPORT_API_BINDINGS, ""),
        ] {
            assert!(result.is_err());
        }
        assert!(has_validator("snapshot-budgets", "fn validate(", ""));
        assert!(!has_validator("snapshot-budgets", "", ""));
        assert!(has_validator(
            "oracle-command",
            "",
            "for oracle in &config.oracles #[serde(deny_unknown_fields)]"
        ));
        assert!(!has_validator("oracle-command", "", ""));
        assert!(!has_validator(
            "oracle-command",
            "",
            "for oracle in &config.oracles"
        ));
        assert!(has_validator("control", "pub fn new(", ""));
        assert!(!has_validator("control", "", ""));
        let mut invalid_object_validator = OBJECT_API_BINDINGS;
        invalid_object_validator[0] = ("control", "pub struct Control");
        let mut invalid_nested_validator = NESTED_API_BINDINGS;
        invalid_nested_validator[0] = ("control", "pub struct Control");
        let mut invalid_report_validator = REPORT_API_BINDINGS;
        invalid_report_validator[0] = ("control", "pub struct Control");
        for result in [
            audit_validators("object", &invalid_object_validator, "impl Control { }"),
            audit_validators("nested", &invalid_nested_validator, "impl Control { }"),
            audit_validators("report-fact", &invalid_report_validator, "impl Control { }"),
        ] {
            assert!(result.is_err());
        }
        let mut empty_object_binding = OBJECT_API_BINDINGS;
        empty_object_binding[0].1 = "";
        let mut empty_nested_binding = NESTED_API_BINDINGS;
        empty_nested_binding[0].1 = "";
        let mut empty_report_binding = REPORT_API_BINDINGS;
        empty_report_binding[0].1 = "";
        for result in [
            audit_validators("object", &empty_object_binding, ""),
            audit_validators("nested", &empty_nested_binding, ""),
            audit_validators("report-fact", &empty_report_binding, ""),
        ] {
            assert!(result.is_err());
        }
        for result in [
            audit_api_bindings(&[], "enum", &ENUM_API_BINDINGS, ""),
            audit_api_bindings(&[], "object", &OBJECT_API_BINDINGS, ""),
            audit_api_bindings(&[], "nested", &NESTED_API_BINDINGS, ""),
            audit_api_bindings(&[], "report-fact", &REPORT_API_BINDINGS, ""),
            audit_api_bindings(&[], "domain", &DOMAIN_API_BINDINGS, ""),
            audit_api_bindings(&rows, "enum", &ENUM_API_BINDINGS, ""),
            audit_api_bindings(&rows, "object", &OBJECT_API_BINDINGS, ""),
            audit_api_bindings(&rows, "nested", &NESTED_API_BINDINGS, ""),
            audit_api_bindings(&rows, "report-fact", &REPORT_API_BINDINGS, ""),
            audit_api_bindings(&rows, "domain", &DOMAIN_API_BINDINGS, ""),
        ] {
            assert!(result.is_err());
        }

        let canonical = read_text(DEFAULT_CANONICAL_VECTORS).expect("canonical vectors");
        let canonical_first = canonical.lines().nth(1).expect("canonical vector");
        for control in [
            canonical.trim_end_matches('\n').to_owned(),
            canonical.replacen(CANONICAL_VECTOR_HEADER, "wrong-header", 1),
            format!("{CANONICAL_VECTOR_HEADER}\nmalformed\n"),
            canonical.replacen(
                canonical_first,
                &format!("{canonical_first}\n{canonical_first}"),
                1,
            ),
            canonical.replacen("\ttype-tag\t", "\tunknown-kind\t", 1),
            canonical.replacen(
                "spec-provenance-canonical-vector/v2\t",
                "wrong-vector-schema\t",
                1,
            ),
        ] {
            assert!(validate_canonical_vector_parity(&rows, &control).is_err());
        }
        let implementation = IMPLEMENTATION_SOURCES
            .iter()
            .map(|path| read_text(path).expect("implementation source"))
            .collect::<Vec<_>>()
            .join("\n");
        for category in ["enum", "object", "nested", "report-fact", "domain"] {
            let mut removed = false;
            let rows_without_category_member = rows
                .iter()
                .filter(|row| {
                    if !removed && row.category == category {
                        removed = true;
                        false
                    } else {
                        true
                    }
                })
                .cloned()
                .collect::<Vec<_>>();
            assert!(
                validate_api_parity_sources(
                    &rows_without_category_member,
                    &implementation,
                    &canonical,
                )
                .is_err()
            );
        }
        let mut invalid_object_rows = rows.clone();
        invalid_object_rows
            .iter_mut()
            .find(|row| row.category == "object")
            .expect("object row")
            .tag = None;
        assert!(
            validate_api_parity_sources(&invalid_object_rows, &implementation, &canonical).is_err()
        );
        let missing_nested_validator =
            implementation.replacen("impl ObjectRef {", "impl ObjectRefWithoutValidator {", 1);
        assert!(validate_api_parity_sources(&rows, &missing_nested_validator, &canonical).is_err());
        let missing_report_validator = implementation.replacen(
            "impl AuthorityFact {",
            "impl AuthorityFactWithoutValidator {",
            1,
        );
        assert!(validate_api_parity_sources(&rows, &missing_report_validator, &canonical).is_err());
        let missing_domain_block = implementation.replacen(
            "impl RepositoryConfig {",
            "impl RepositoryConfig /* missing block */ {",
            1,
        );
        assert!(validate_api_parity_sources(&rows, &missing_domain_block, &canonical).is_err());

        let mut impact_cases = BTreeMap::new();
        insert_impact_case(
            &mut impact_cases,
            String::from("control"),
            "positive",
            "edge",
            "source",
            "target",
            "target",
            1,
            "none",
            "accept",
            "-",
            "target",
        )
        .expect("impact case");
        assert!(
            insert_impact_case(
                &mut impact_cases,
                String::from("control"),
                "negative",
                "edge",
                "source",
                "target",
                "target",
                1,
                "mutation",
                "reject",
                "CHG-CONTROL",
                "target",
            )
            .is_err()
        );
        assert!(require_impact_case(&impact_cases, "missing", "positive", "accept").is_err());
        let mut malformed_impact_cases = impact_cases.clone();
        malformed_impact_cases.insert(String::from("control"), String::from("malformed\n"));
        assert!(
            require_impact_case(&malformed_impact_cases, "control", "positive", "accept").is_err()
        );
        assert!(require_impact_case(&impact_cases, "control", "negative", "accept").is_err());
    }
}
