#![doc = "Executable contracts for the public Buoy examples."]
#![allow(clippy::expect_used, clippy::too_many_lines)]

use buoy::adapters::{
    AuthorityExpectation, OpenSpecConfiguration, extract_markdown, extract_openspec_configured,
};
use buoy::core::{RepositoryPath, SemanticDigest};
use buoy::repository::{RepositoryConfig, RepositoryError};
#[path = "support/traceability_control.rs"]
mod traceability_control;
#[cfg(unix)]
use buoy::{check::check_snapshot, repository::snapshot_repository};
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};
#[cfg(unix)]
use std::process::{Command, Output};
use tempfile::TempDir;

const MANIFEST_HEADER: &str = "case_id\trequirement\tpolarity\tkind\tinput\tcommand\texpected_exit\texpected_diagnostic\texpected_output";

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
enum Polarity {
    Positive,
    Negative,
}

#[derive(Debug)]
struct ExampleCase {
    id: String,
    requirement: String,
    polarity: Polarity,
    input: PathBuf,
    command: String,
    expected_exit: u8,
    expected_diagnostic: Option<String>,
    expected_output: Option<PathBuf>,
}

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn read(path: impl AsRef<Path>) -> String {
    fs::read_to_string(path).expect("example text must be readable")
}

fn parse_cases(source: &str) -> Result<Vec<ExampleCase>, String> {
    let mut lines = source.lines();
    if lines.next() != Some(MANIFEST_HEADER) {
        return Err(String::from("EXAMPLE-MANIFEST-HEADER"));
    }
    let mut cases = Vec::new();
    let mut identities = BTreeSet::new();
    for (index, line) in lines.enumerate() {
        let columns = line.split('\t').collect::<Vec<_>>();
        if columns.len() != 9 || columns.iter().any(|column| column.is_empty()) {
            return Err(format!("EXAMPLE-MANIFEST-ROW:{}", index + 2));
        }
        let polarity = match columns[2] {
            "positive" => Polarity::Positive,
            "negative" => Polarity::Negative,
            _ => return Err(format!("EXAMPLE-MANIFEST-POLARITY:{}", columns[0])),
        };
        let expected_exit = columns[6]
            .parse::<u8>()
            .map_err(|_| format!("EXAMPLE-MANIFEST-EXIT:{}", columns[0]))?;
        if !identities.insert(columns[0].to_owned()) {
            return Err(format!("EXAMPLE-MANIFEST-DUPLICATE:{}", columns[0]));
        }
        cases.push(ExampleCase {
            id: columns[0].to_owned(),
            requirement: columns[1].to_owned(),
            polarity,
            input: PathBuf::from(columns[4]),
            command: columns[5].to_owned(),
            expected_exit,
            expected_diagnostic: (columns[7] != "none").then(|| columns[7].to_owned()),
            expected_output: (columns[8] != "none").then(|| PathBuf::from(columns[8])),
        });
    }
    Ok(cases)
}

fn require_example_files(repository: &Path) -> Result<(), &'static str> {
    for relative in [
        "examples/README.md",
        "examples/core_graph.rs",
        "examples/repository_check.rs",
        "examples/manifest.tsv",
    ] {
        if !repository.join(relative).is_file() {
            return Err("EXAMPLE-FILE-MISSING");
        }
    }
    Ok(())
}

#[test]
fn example_manifest_covers_every_positive_and_negative_scenario() {
    let source = read(root().join("examples/manifest.tsv"));
    let cases = parse_cases(&source).expect("reviewed example manifest");
    assert_eq!(cases.len(), 30, "EXAMPLE-CASE-DRIFT");
    assert_eq!(
        cases
            .iter()
            .filter_map(|case| case.expected_diagnostic.as_deref())
            .collect::<BTreeSet<_>>(),
        BTreeSet::from([
            "CHG-DISPOSITION-MISSING",
            "EXAMPLE-ARTIFACT-ORDER",
            "EXAMPLE-ASSURANCE-BOUNDARY",
            "EXAMPLE-CASE-DRIFT",
            "EXAMPLE-INTERNAL-RULE",
            "EXAMPLE-PROVIDER-STATE",
            "EXAMPLE-REQUIREMENT-ID",
            "EXAMPLE-SCHEMA-DRIFT",
            "EXAMPLE-TEMPLATE-AUTHORITY",
            "EXAMPLE-TRACE-PAGE-DRIFT",
            "REPO-CONFIG-SCHEMA",
            "SPG-OPENSPEC-STANDING-PATH",
            "SPG-REF-DANGLING-OR-STALE",
        ]),
        "EXAMPLE-DIAGNOSTIC-DRIFT"
    );
    let repository = root();
    for case in &cases {
        assert!(repository.join(&case.input).is_file(), "{}", case.id);
        assert!(!case.command.is_empty(), "{}", case.id);
        if let Some(output) = &case.expected_output {
            assert!(repository.join(output).is_file(), "{}", case.id);
        }
        match case.polarity {
            Polarity::Positive => assert!(case.expected_diagnostic.is_none(), "{}", case.id),
            Polarity::Negative => assert!(case.expected_diagnostic.is_some(), "{}", case.id),
        }
        assert!(matches!(case.expected_exit, 0 | 1 | 2 | 3 | 64 | 70));
    }
    let coverage = cases.into_iter().fold(
        BTreeMap::<String, BTreeSet<Polarity>>::new(),
        |mut result, case| {
            result
                .entry(case.requirement)
                .or_default()
                .insert(case.polarity);
            result
        },
    );
    for prefix in ["BUE", "BOT"] {
        for index in 1..=7 {
            let requirement = format!("{prefix}-{index:03}");
            assert_eq!(
                coverage.get(&requirement),
                Some(&BTreeSet::from([Polarity::Positive, Polarity::Negative]))
            );
        }
    }
}

#[test]
fn required_example_file_control_fails_closed() {
    let absent = TempDir::new().expect("absent example root");
    assert_eq!(
        require_example_files(absent.path()),
        Err("EXAMPLE-FILE-MISSING")
    );
    assert!(require_example_files(&root()).is_ok());
}

#[test]
fn malformed_configuration_is_rejected_at_the_parse_boundary() {
    assert_eq!(
        RepositoryConfig::parse(include_bytes!(
            "../examples/repositories/invalid-config.toml"
        )),
        Err(RepositoryError::ConfigSchema)
    );
}

#[test]
fn openspec_authority_and_active_path_controls_are_exact() {
    let repository_configuration = RepositoryConfig::parse(include_bytes!(
        "../examples/repositories/openspec-authority/buoy.toml"
    ))
    .expect("example repository configuration");
    let adapter_configuration = OpenSpecConfiguration::parse(
        repository_configuration
            .adapter_configuration("openspec/v1")
            .expect("OpenSpec adapter configuration"),
    )
    .expect("example OpenSpec configuration");
    let standing_path = RepositoryPath::new("openspec/specs/demo/spec.md").expect("standing path");
    let bytes =
        include_bytes!("../examples/repositories/openspec-authority/openspec/specs/demo/spec.md");
    let standing = extract_openspec_configured(&standing_path, bytes, &adapter_configuration);
    assert!(standing.diagnostics.is_empty());
    assert_eq!(standing.semantic.len(), 3);
    let active_path = RepositoryPath::new("openspec/changes/demo/spec.md").expect("active path");
    let active_bytes = include_bytes!(
        "../examples/repositories/openspec-active-change/openspec/changes/demo/spec.md"
    );
    assert_eq!(
        active_bytes.as_slice(),
        bytes.as_slice(),
        "EXAMPLE-ACTIVE-CONTROL-DRIFT"
    );
    let active = extract_openspec_configured(&active_path, active_bytes, &adapter_configuration);
    assert_eq!(active.diagnostics[0].code, "SPG-OPENSPEC-STANDING-PATH");
    assert!(active.semantic.is_empty());
}

#[test]
fn external_relation_example_contains_only_the_closed_navigation_set() {
    let path = RepositoryPath::new("docs/all-relations.md").expect("trace example path");
    let output = extract_markdown(
        &path,
        include_bytes!("../examples/repositories/openspec-authority/docs/all-relations.md"),
        AuthorityExpectation::Optional,
    );
    assert!(output.diagnostics.is_empty());
    assert_eq!(output.external_links.len(), 4);
    let repository_configuration = RepositoryConfig::parse(include_bytes!(
        "../examples/repositories/openspec-authority/buoy.toml"
    ))
    .expect("example repository configuration");
    let adapter_configuration = OpenSpecConfiguration::parse(
        repository_configuration
            .adapter_configuration("openspec/v1")
            .expect("OpenSpec adapter configuration"),
    )
    .expect("example OpenSpec configuration");
    let standing = extract_openspec_configured(
        &RepositoryPath::new("openspec/specs/demo/spec.md").expect("standing path"),
        include_bytes!("../examples/repositories/openspec-authority/openspec/specs/demo/spec.md"),
        &adapter_configuration,
    );
    let atom = standing
        .semantic
        .iter()
        .find_map(|located| located.value.atom())
        .expect("standing requirement atom");
    for link in &output.external_links {
        assert_eq!(link.value.atoms().len(), 1, "EXAMPLE-TRACE-ATOM-SET");
        assert_eq!(link.value.atoms()[0].id(), atom.meta().id());
        assert_eq!(link.value.atoms()[0].revision(), atom.revision());
    }
    assert_eq!(
        output
            .external_links
            .iter()
            .map(|link| link.value.relation().as_str())
            .collect::<BTreeSet<_>>(),
        BTreeSet::from(["discusses", "documents", "reviews", "tracks"])
    );
    assert!(output.semantic.is_empty());
}

#[test]
fn reusable_schema_and_templates_preserve_the_buoy_boundary() {
    let base = root().join("examples/openspec-workflow/openspec");
    let schema = read(base.join("schemas/buoy-traceability/schema.yaml"));
    for required in [
        "name: buoy-traceability",
        "- id: proposal",
        "- id: specs",
        "- id: design",
        "- id: traceability",
        "- id: tasks",
        "      - traceability",
    ] {
        assert!(
            schema.contains(required),
            "EXAMPLE-SCHEMA-DRIFT: {required}"
        );
    }
    let order = ["proposal", "specs", "design", "traceability", "tasks"].map(|artifact| {
        schema
            .find(&format!("- id: {artifact}"))
            .expect("required artifact")
    });
    assert!(
        order.windows(2).all(|pair| pair[0] < pair[1]),
        "EXAMPLE-ARTIFACT-ORDER"
    );

    let specification = read(base.join("schemas/buoy-traceability/templates/specs/spec.md"));
    for required in [
        "### Requirement: DEMO-001",
        "MUST",
        "Scenario: Positive",
        "Scenario: Negative",
        "**GIVEN**",
        "**WHEN**",
        "**THEN**",
    ] {
        assert!(
            specification.contains(required),
            "EXAMPLE-REQUIREMENT-ID: {required}"
        );
    }
    let traceability = read(base.join("schemas/buoy-traceability/templates/traceability.md"));
    for required in [
        "Requirement ID",
        "Target class",
        "Action",
        "Owner",
        "Focused Buoy command",
        "Link Review",
        "External Targets",
        "Exclude provider status",
        "Planned Implementation Relations",
        "Relation ID",
        "Logical target selector",
        "Completion Snapshot",
        "<!-- BEGIN buoy-trace-publication-evidence-v1 -->",
        "buoy publication evidence",
        "Review decision",
        "Buoy remains the current-state authority",
        "Do not copy the immutable record body",
    ] {
        assert!(
            traceability.contains(required),
            "EXAMPLE-PROVIDER-STATE: {required}"
        );
    }
    let design = read(base.join("schemas/buoy-traceability/templates/design.md"));
    for required in [
        "Repository configuration owns authority",
        "Active OpenSpec changes are noncanonical plans",
        "Trust Boundaries",
    ] {
        assert!(
            design.contains(required),
            "EXAMPLE-TEMPLATE-AUTHORITY: {required}"
        );
    }
    let project_text = [
        read(base.join("config.yaml")),
        schema,
        read(base.join("schemas/buoy-traceability/templates/proposal.md")),
        specification,
        design,
        traceability,
        read(base.join("schemas/buoy-traceability/templates/tasks.md")),
    ]
    .join("\n");
    for forbidden in [
        "Verus",
        "AeneasVerif",
        "Hegel",
        "devenv test",
        "assurance/v1",
        "proofs/",
        "provider_status =",
        "fetched_title =",
    ] {
        assert!(
            !project_text.contains(forbidden),
            "EXAMPLE-INTERNAL-RULE: {forbidden}"
        );
    }
}

#[cfg(unix)]
fn run_buoy(arguments: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_buoy"))
        .args(arguments)
        .output()
        .expect("Buoy example command must execute")
}

#[cfg(unix)]
fn path_text(path: &Path) -> String {
    path.to_string_lossy().into_owned()
}

/// TPT: the `buoy publication` usage text must name its required
/// `--config` and repository-root (`--root`/`--mount`) inputs explicitly --
/// a publication operation is never a report-only stub that runs without a
/// real repository. This runs the real CLI binary with no arguments and
/// inspects its real emitted usage text on standard error.
#[cfg(unix)]
#[test]
fn publication_usage_text_names_required_configuration_and_root_inputs() {
    let output = run_buoy(&[]);
    assert_eq!(output.status.code(), Some(64));
    let usage = String::from_utf8(output.stderr).expect("UTF-8 usage text");
    let publication_lines: Vec<&str> = usage
        .lines()
        .filter(|line| line.contains("buoy publication"))
        .collect();
    assert!(
        !publication_lines.is_empty(),
        "usage text must include a `buoy publication` line: {usage}"
    );
    for line in &publication_lines {
        assert!(
            line.contains("--config FILE"),
            "publication usage line must require --config: {line}"
        );
        assert!(
            line.contains("--root ROOT") && line.contains("--mount ID=PATH"),
            "publication usage line must require --root or --mount: {line}"
        );
    }
}

#[cfg(unix)]
#[test]
fn command_examples_match_exact_outputs_and_exit_classes() {
    let repository = root();
    let authority = repository.join("examples/repositories/openspec-authority");
    let authority_config = authority.join("buoy.toml");
    let authority_output = run_buoy(&[
        "static",
        "--root",
        &path_text(&authority),
        "--config",
        &path_text(&authority_config),
        "--format",
        "machine",
    ]);
    assert_eq!(authority_output.status.code(), Some(0));
    assert_eq!(
        authority_output.stdout,
        fs::read(repository.join("examples/expected/openspec-static.json")).expect("static golden")
    );

    let stale = repository.join("examples/repositories/stale-external-link");
    let stale_output = run_buoy(&[
        "static",
        "--root",
        &path_text(&stale),
        "--config",
        &path_text(&stale.join("buoy.toml")),
        "--format",
        "machine",
    ]);
    assert_eq!(stale_output.status.code(), Some(2));
    assert_eq!(
        stale_output.stdout,
        fs::read(repository.join("examples/expected/stale-static.json")).expect("stale golden")
    );

    let change = repository.join("examples/repositories/change-check");
    for (manifest, expected_status, golden) in [
        ("complete.toml", 0, "change-complete.json"),
        ("incomplete.toml", 3, "change-incomplete.json"),
    ] {
        let output = run_buoy(&[
            "change",
            "--base",
            &path_text(&change.join("base")),
            "--head",
            &path_text(&change.join("head")),
            "--config",
            &path_text(&change.join("buoy.toml")),
            "--manifest",
            &path_text(&change.join(manifest)),
            "--format",
            "machine",
        ]);
        assert_eq!(output.status.code(), Some(expected_status));
        assert_eq!(
            output.stdout,
            fs::read(repository.join("examples/expected").join(golden)).expect("change golden")
        );
    }
}

#[cfg(unix)]
#[test]
fn committed_change_manifests_bind_recomputed_graph_and_atom_digests() {
    let repository = root();
    let example = repository.join("examples/repositories/change-check");
    let config = RepositoryConfig::parse(
        &fs::read(example.join("buoy.toml")).expect("change configuration"),
    )
    .expect("change configuration parse");
    let checked = |name: &str| {
        let snapshot = snapshot_repository(&example.join(name), &config).expect("example snapshot");
        check_snapshot(&snapshot, &config).expect("example static check")
    };
    let base = checked("base");
    let head = checked("head");
    assert_eq!(base.graph.atoms.len(), 1);
    assert_eq!(head.graph.atoms.len(), 1);
    let base_atom = &base.graph.atoms[0];
    let head_atom = &head.graph.atoms[0];
    let complete = format!(
        "schema = \"spec-provenance-change-manifest/v1\"\nid = \"CHANGE-EXAMPLE\"\nbase_graph = \"{}\"\nhead_graph = \"{}\"\n\n[[dispositions]]\nid = \"CHANGE-EXAMPLE-DISPOSITION\"\nkind = \"atom\"\nobject = \"CHANGE-EXAMPLE-001\"\nbase = \"{}\"\nhead = \"{}\"\ndisposition = \"revised\"\n",
        base.graph_digest,
        head.graph_digest,
        base_atom.meta().digest(),
        head_atom.meta().digest(),
    );
    assert_eq!(read(example.join("complete.toml")), complete);
    let incomplete = format!(
        "schema = \"spec-provenance-change-manifest/v1\"\nid = \"CHANGE-EXAMPLE-INCOMPLETE\"\nbase_graph = \"{}\"\nhead_graph = \"{}\"\ndispositions = []\n",
        base.graph_digest, head.graph_digest,
    );
    assert_eq!(read(example.join("incomplete.toml")), incomplete);
    assert_eq!(
        SemanticDigest::parse_canonical(&base.graph_digest.to_string()),
        Ok(base.graph_digest)
    );
}

#[test]
fn completed_traceability_example_passes_the_workflow_control() {
    let example = read(root().join("examples/openspec-workflow/completed-traceability.md"));
    // The shipped example is an ACTIVE workflow artifact, so it is validated
    // under the strict lifecycle: it must carry the marker-delimited
    // generated evidence block, not merely avoid contradicting one.
    let faults = traceability_control::validate_active_traceability(&example)
        .into_iter()
        .map(traceability_control::TraceabilityFault::name)
        .collect::<Vec<_>>();
    assert!(
        faults.is_empty(),
        "the completed workflow example must contain no fault: {faults:?}"
    );
}

/// The active-artifact requirement must actually bite: deleting ANY part of
/// the completion structure from the shipped example has to be detected under
/// its own identity. Without this the requirement could be satisfied
/// vacuously by an example that happens to carry the block, and -- the actual
/// regression -- deleting the whole `## Completion Snapshot` heading used to
/// skip every completion rule instead of failing.
#[test]
fn an_active_artifact_missing_any_completion_block_is_detected() {
    let example = read(root().join("examples/openspec-workflow/completed-traceability.md"));
    let active = |text: &str| {
        traceability_control::validate_active_traceability(text)
            .into_iter()
            .map(traceability_control::TraceabilityFault::name)
            .collect::<Vec<_>>()
    };
    // The unmutated example passes here, so each failure below is
    // attributable to its own mutation.
    assert!(
        active(&example).is_empty(),
        "the shipped example must pass the active control unmutated"
    );
    for (case, mutated, expected) in [
        (
            "heading-deleted",
            example.replace("## Completion Snapshot\n", ""),
            "missing-completion-section",
        ),
        (
            "generated-block-deleted",
            example
                .replace("<!-- BEGIN buoy-trace-publication-evidence-v1 -->\n", "")
                .replace("<!-- END buoy-trace-publication-evidence-v1 -->\n", ""),
            "missing-generated-block",
        ),
        (
            // An unterminated block is equally unsafe: the generated region
            // would be unbounded, letting later prose read as evidence.
            "end-marker-truncated",
            example.replace("<!-- END buoy-trace-publication-evidence-v1 -->\n", ""),
            "unterminated-generated-block",
        ),
        (
            "human-block-deleted",
            example.replace(
                "| Owner | Consumer commit | Review decision |\n|---|---|---|\n",
                "",
            ),
            "missing-human-completion-block",
        ),
    ] {
        assert_ne!(mutated, example, "{case}: the mutation must change bytes");
        let faults = active(&mutated);
        assert!(
            faults.contains(&expected),
            "{case}: an active artifact must report {expected}: {faults:?}"
        );
        // An ARCHIVED artifact legitimately predates this structure and must
        // stay readable, so the two lifecycles really do differ.
        let archived = traceability_control::validate_traceability(&mutated)
            .into_iter()
            .map(traceability_control::TraceabilityFault::name)
            .collect::<Vec<_>>();
        assert!(
            !archived.contains(&expected),
            "{case}: an archived artifact must remain readable: {archived:?}"
        );
    }
}

/// The example workflow template a consumer copies must itself satisfy the
/// active control and declare BOTH completion blocks.
#[test]
fn the_example_workflow_template_declares_both_completion_blocks() {
    let template = read(root().join(
        "examples/openspec-workflow/openspec/schemas/buoy-traceability/templates/traceability.md",
    ));
    let completion = template
        .split_once("## Completion Snapshot")
        .expect("completion section")
        .1;
    assert!(completion.contains("<!-- BEGIN buoy-trace-publication-evidence-v1 -->"));
    assert!(completion.contains("<!-- END buoy-trace-publication-evidence-v1 -->"));
    assert!(completion.contains("| Owner | Consumer commit | Review decision |"));
    let faults = traceability_control::validate_active_traceability(&template)
        .into_iter()
        .map(traceability_control::TraceabilityFault::name)
        .collect::<Vec<_>>();
    assert!(
        faults.is_empty(),
        "the example template must pass the active control: {faults:?}"
    );
}

/// The implementation-owned trace page must regenerate byte-for-byte.
///
/// The page lives outside every authority root. It is derived navigation data
/// and supplies no coverage, execution, proof, approval, or correctness.
#[cfg(all(unix, feature = "blake3-256"))]
#[test]
fn the_generated_trace_page_matches_its_current_query() {
    let repository = root();
    let fixture = repository.join("fixtures/readable-trace-projections/v1/repository");
    let output = Command::new(env!("CARGO_BIN_EXE_buoy"))
        .args([
            "trace",
            "--root",
            fixture.to_str().expect("UTF-8 fixture root"),
            "--config",
            fixture
                .join("buoy.toml")
                .to_str()
                .expect("UTF-8 fixture config"),
            "--relation-id",
            "BGLC-RENDER-CATALOG",
            "--format",
            "markdown",
        ])
        .output()
        .expect("trace page command");
    assert_eq!(output.status.code(), Some(0));
    let generated = String::from_utf8(output.stdout).expect("UTF-8 trace page");
    assert_eq!(
        generated,
        read(repository.join("examples/expected/trace-page.md")),
        "EXAMPLE-TRACE-PAGE-DRIFT: regenerate with buoy trace --format markdown"
    );

    // The page states its derived scope and keeps the navigation boundary.
    for required in [
        "Derived data",
        "Buoy remains the current-state authority",
        "navigation and impact only",
    ] {
        assert!(
            generated.contains(required),
            "EXAMPLE-ASSURANCE-BOUNDARY: {required}"
        );
    }
    // A generated page never sits under one selected authority root.
    assert!(!repository.join("openspec/specs/trace-page.md").exists());
}
