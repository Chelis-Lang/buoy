//! Validate complete source scope and exact LLVM line, region, function, and branch counts.

#![allow(
    clippy::assigning_clones,
    clippy::expect_used,
    clippy::too_many_lines,
    clippy::unnecessary_wraps
)]

use serde::Deserialize;
use serde_json::Value;
use std::collections::{BTreeMap, BTreeSet};
use std::env;
use std::fs;
use std::path::{Component, Path};

#[path = "digest_binding.rs"]
mod content_digest;

const INVENTORY: &str = "assurance/v1/source-inventory.tsv";
const FIXTURE_ROOT: &str = "fixtures/coverage/v1";
const INVENTORY_HEADER: &str =
    "target_id\tpath\tkind\tplatform\tcoverage_class\tdisposition\trationale";
const RESULT_HEADER: &str = "schema\ttarget_id\tpath\tsource_digest\tplatform\tlines_covered\tlines_count\tregions_covered\tregions_count\tfunctions_covered\tfunctions_count\tbranches_covered\tbranches_count\ttool_digest\tconfiguration_digest\tinventory_digest\texport_digest\tregistration\tfreshness\texecution\tassurance_class";
const FIXTURE_MANIFEST_HEADER: &str = "fixture_id\tkind\tpolarity\tinventory\texport\tmetadata\tresult\texpected_diagnostic\texpected_count";
const METRICS: [&str; 4] = ["lines", "regions", "functions", "branches"];
const COVERED_DISPOSITIONS: [&str; 4] = [
    "included",
    "included-assurance",
    "included-manual",
    "included-assurance-manual",
];
const COVERAGE_CLASSES: [&str; 6] = [
    "cargo-llvm-cov",
    "cargo-harness-llvm-cov",
    "focused-standalone-cargo-llvm-cov",
    "example-cargo-llvm-cov",
    "aggregate-platform-cargo-llvm-cov",
    "separate-manifest-cargo-llvm-cov",
];
const VALID_CLASSES: [&str; 13] = [
    "cargo-llvm-cov",
    "cargo-harness-llvm-cov",
    "focused-standalone-cargo-llvm-cov",
    "example-cargo-llvm-cov",
    "aggregate-platform-cargo-llvm-cov",
    "separate-manifest-cargo-llvm-cov",
    "compiled-data-and-conformance",
    "no-executable-regions",
    "fixture-execution",
    "formal-binding",
    "formal-spike",
    "derivation-and-formal-spike",
    "fixture-validation",
];
const DISCOVERY_ROOTS: [&str; 7] = [
    "src",
    "crates",
    "tests",
    "examples",
    "tools",
    "dylint_lints",
    "fixtures/dylint",
];

type CheckResult<T> = Result<T, String>;

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
struct Diagnostic {
    code: String,
    subject: String,
    message: String,
}

impl Diagnostic {
    fn new(code: &str, subject: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            code: code.to_owned(),
            subject: subject.into(),
            message: message.into(),
        }
    }

    fn render(&self) -> String {
        format!("{} [{}]: {}", self.code, self.subject, self.message)
    }
}

#[derive(Clone, Debug)]
struct InventoryRow {
    target_id: String,
    path: String,
    _kind: String,
    platforms: BTreeSet<String>,
    coverage_class: String,
    disposition: String,
    _rationale: String,
}

#[derive(Clone, Debug, Deserialize)]
struct ExportBinding {
    schema: String,
    export_id: String,
    tool_id: String,
    tool_version: String,
    tool_digest: String,
    configuration_digest: String,
    platform: String,
    inventory_digest: String,
    execution: String,
}

#[derive(Clone, Debug)]
struct ExpectedBinding {
    tool_digest: String,
    configuration_digest: String,
    inventory_digest: String,
    platform: String,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct MetricCounts {
    count: u64,
    covered: u64,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct ExportFile {
    path: String,
    metrics: BTreeMap<String, MetricCounts>,
    segments: Vec<String>,
}

#[derive(Clone, Debug)]
struct ParsedExport {
    files: BTreeMap<String, ExportFile>,
    totals: BTreeMap<String, MetricCounts>,
}

fn main() {
    std::process::exit(exit_code(run()));
}

fn exit_code(result: CheckResult<()>) -> i32 {
    match result {
        Ok(()) => 0,
        Err(error) => {
            eprintln!("exact-coverage: {error}");
            1
        }
    }
}

fn run() -> CheckResult<()> {
    let arguments = env::args().skip(1).collect::<Vec<_>>();
    dispatch(&arguments)
}

fn dispatch(arguments: &[String]) -> CheckResult<()> {
    match arguments {
        [] => validate_authoritative_inventory(),
        [argument] if argument == "--self-test" => self_test(),
        [argument] if argument == "--check-inventory" => validate_authoritative_inventory(),
        [argument, result] if argument == "--check-result" => check_result_path(result),
        [argument, source_root, inventory, export, metadata]
            if argument == "--validate-fixture" =>
        {
            let binding = parse_binding(metadata)?;
            let expected = ExpectedBinding {
                tool_digest: binding.tool_digest.clone(),
                configuration_digest: binding.configuration_digest.clone(),
                inventory_digest: binding.inventory_digest.clone(),
                platform: binding.platform.clone(),
            };
            emit_diagnostics(validate_export_set(
                Path::new(source_root),
                "/fixture",
                inventory,
                export,
                &binding,
                &expected,
                false,
            )?)
        }
        [argument, path] if argument == "--sha256" => {
            content_digest::self_test().expect("content-digest self-test must pass");
            content_digest::file_binding(Path::new(path)).map(|binding| println!("{binding}"))
        }
        [
            argument,
            export_id,
            platform,
            tool,
            configuration,
            inventory,
        ] if argument == "--binding" => {
            println!(
                "schema = \"buoy-coverage-export-binding/v1\"\nexport_id = \"{export_id}\"\ntool_id = \"cargo-llvm-cov\"\ntool_version = \"0.8.5\"\ntool_digest = \"{}\"\nconfiguration_digest = \"{}\"\nplatform = \"{platform}\"\ninventory_digest = \"{}\"\nexecution = \"pass\"",
                content_digest::file_binding(Path::new(tool))?,
                content_digest::file_binding(Path::new(configuration))?,
                content_digest::file_binding(Path::new(inventory))?,
            );
            Ok(())
        }
        [argument, source_root, inventory, rest @ ..]
            if argument == "--normalize" && rest.len() >= 2 && rest.len() % 2 == 0 =>
        {
            finish_normalization(normalize_exports(
                Path::new(source_root),
                inventory,
                rest,
                true,
            ))
        }
        [argument, source_root, inventory, result, rest @ ..]
            if argument == "--check-current" && rest.len() >= 2 && rest.len() % 2 == 0 =>
        {
            finish_current_result(
                normalize_exports(Path::new(source_root), inventory, rest, true),
                result,
            )
        }
        [argument, inventory, _, ..] if argument == "--check-ownership" => {
            check_ownership(inventory, &arguments[2..])
        }
        [report_path] => validate_legacy_report(report_path),
        _ => Err(String::from(
            "usage: check-exact-coverage [--self-test|--check-inventory|--check-result <result>|--sha256 <file>|--binding <export-id> <platform> <tool> <configuration> <inventory>|--normalize <source-root> <inventory> <metadata> <export>...|--check-current <source-root> <inventory> <result> <metadata> <export>...|--check-ownership <inventory> <metadata>...|--validate-fixture <source-root> <inventory> <export> <metadata>|<llvm-export>]",
        )),
    }
}

fn check_result_path(path: &str) -> CheckResult<()> {
    read(path)
        .and_then(|source| validate_result(&source))
        .and_then(emit_diagnostics)
}

fn finish_normalization(normalization: CheckResult<(Vec<Diagnostic>, String)>) -> CheckResult<()> {
    let (diagnostics, result) = normalization?;
    emit_diagnostics(diagnostics)?;
    print!("{result}");
    Ok(())
}

fn finish_current_result(
    normalization: CheckResult<(Vec<Diagnostic>, String)>,
    result_path: &str,
) -> CheckResult<()> {
    let (diagnostics, normalized) = normalization?;
    emit_diagnostics(diagnostics)?;
    let current = read(result_path)?;
    emit_diagnostics(validate_current_result(&current, &normalized))?;
    println!("exact-coverage: current result ok");
    Ok(())
}

fn finish_legacy_report(diagnostics: Vec<Diagnostic>) -> CheckResult<()> {
    emit_diagnostics(diagnostics)?;
    println!("exact-coverage: report ok line=region=function=branch=100");
    Ok(())
}

fn validate_authoritative_inventory() -> CheckResult<()> {
    read(INVENTORY).and_then(|source| validate_authoritative_inventory_source(&source))
}

fn validate_authoritative_inventory_source(source: &str) -> CheckResult<()> {
    let (rows, mut diagnostics) = parse_inventory(source, Path::new("."), true);
    if diagnostics.is_empty() {
        diagnostics.extend(validate_discovered_sources(&rows));
    }
    emit_diagnostics(diagnostics)?;
    println!("exact-coverage: inventory ok targets={}", rows.len());
    Ok(())
}

fn validate_legacy_report(report_path: &str) -> CheckResult<()> {
    read(INVENTORY).and_then(|source| validate_legacy_report_with_inventory(report_path, &source))
}

fn validate_legacy_report_with_inventory(
    report_path: &str,
    inventory_source: &str,
) -> CheckResult<()> {
    let (rows, mut diagnostics) = parse_inventory(inventory_source, Path::new("."), true);
    if diagnostics.is_empty() {
        diagnostics.extend(validate_discovered_sources(&rows));
        let report = read(report_path)?;
        let parsed = parse_export(
            &report,
            &env::current_dir()
                .expect("validator process must have a current directory")
                .display()
                .to_string(),
        )?;
        diagnostics.extend(validate_source_presence(&rows, &parsed, None));
        diagnostics.extend(validate_metrics(&parsed));
    }
    finish_legacy_report(diagnostics)
}

fn self_test() -> CheckResult<()> {
    content_digest::self_test().expect("content-digest self-test must pass");
    let manifest_path = Path::new(FIXTURE_ROOT).join("manifest.tsv");
    let manifest = read_path(&manifest_path).expect("fixture manifest must read");
    let mut lines = manifest.lines();
    require_conditions(
        &[lines.next() == Some(FIXTURE_MANIFEST_HEADER)],
        "LCOV-NEGATIVE-CONTROL: fixture manifest header mismatch",
    )
    .expect("checked-in fixture manifest header must be valid");
    let positive_binding = parse_binding(
        &Path::new(FIXTURE_ROOT)
            .join("metadata/positive.toml")
            .display()
            .to_string(),
    )
    .expect("positive fixture binding must parse");
    let expected = ExpectedBinding {
        tool_digest: positive_binding.tool_digest,
        configuration_digest: positive_binding.configuration_digest,
        inventory_digest: positive_binding.inventory_digest,
        platform: positive_binding.platform,
    };
    let source_root = Path::new(FIXTURE_ROOT).join("repository");
    let mut checked = 0_usize;
    for (index, line) in lines.enumerate() {
        let columns = line.split('\t').collect::<Vec<_>>();
        require_conditions(
            &[columns.len() == 9],
            &format!(
                "LCOV-NEGATIVE-CONTROL: fixture manifest line {} malformed",
                index + 2
            ),
        )
        .expect("checked-in fixture manifest rows must be valid");
        let fixture_id = columns[0];
        let kind = columns[1];
        let polarity = columns[2];
        let inventory = fixture_path(columns[3]);
        let export = fixture_path(columns[4]);
        let metadata = fixture_path(columns[5]);
        let result = columns[6];
        let expected_code = columns[7];
        let expected_count = parse_expected_count(fixture_id, columns[8])
            .expect("fixture expected count must be an integer");
        let diagnostics = fixture_diagnostics(
            fixture_id,
            kind,
            &source_root,
            &inventory,
            &export,
            &metadata,
            result,
            &expected,
        )
        .expect("fixture validation must complete");
        assert_fixture(
            fixture_id,
            polarity,
            expected_code,
            expected_count,
            &diagnostics,
        )
        .expect("fixture diagnostic must match its manifest declaration");
        checked += 1;
    }
    let normalize_inputs = vec![
        fixture_path("metadata/positive.toml"),
        fixture_path("exports/positive.json"),
    ];
    let (normalization_diagnostics, normalized) = normalize_exports(
        &source_root,
        &fixture_path("inventories/positive.tsv"),
        &normalize_inputs,
        false,
    )
    .expect("positive fixture normalization must complete");
    require_conditions(
        &[normalization_diagnostics.is_empty()],
        &format!(
            "LCOV-NEGATIVE-CONTROL: positive normalization failed: {}",
            render_diagnostics(&normalization_diagnostics)
        ),
    )
    .expect("positive normalization must succeed");
    let result_diagnostics =
        validate_result(&normalized).expect("normalized fixture result must parse");
    require_conditions(
        &[result_diagnostics.is_empty()],
        &format!(
            "LCOV-NEGATIVE-CONTROL: normalized result failed: {}",
            render_diagnostics(&result_diagnostics)
        ),
    )
    .expect("normalized result must validate");
    let duplicate_inputs = [normalize_inputs.clone(), normalize_inputs].concat();
    let (duplicate_diagnostics, duplicate_normalized) = normalize_exports(
        &source_root,
        &fixture_path("inventories/positive.tsv"),
        &duplicate_inputs,
        false,
    )
    .expect("duplicate fixture normalization must complete");
    require_conditions(
        &[
            duplicate_diagnostics.is_empty(),
            duplicate_normalized == normalized,
        ],
        &format!(
            "LCOV-NEGATIVE-CONTROL: duplicate export changed normalized evidence: {}",
            render_diagnostics(&duplicate_diagnostics)
        ),
    )
    .expect("duplicate exports must normalize idempotently");
    require_conditions(
        &[validate_current_result(&normalized, &normalized).is_empty()],
        "LCOV-NEGATIVE-CONTROL: identical normalized result was marked stale",
    )
    .expect("identical normalized results must be current");
    let stale = normalized.replacen("\tcurrent\tpass\t", "\thistorical\tpass\t", 1);
    let stale_diagnostics = validate_current_result(&stale, &normalized);
    require_conditions(
        &[
            stale_diagnostics.len() == 1,
            stale_diagnostics.first().map(|value| value.code.as_str())
                == Some("LCOV-EVIDENCE-STALE"),
        ],
        "LCOV-NEGATIVE-CONTROL: changed normalized result did not become stale exactly once",
    )
    .expect("changed normalized results must be stale exactly once");

    let reordered_inputs = vec![
        fixture_path("metadata/positive.toml"),
        fixture_path("exports/positive-reordered.json"),
    ];
    let (reordered_diagnostics, reordered_normalized) = normalize_exports(
        &source_root,
        &fixture_path("inventories/positive.tsv"),
        &reordered_inputs,
        false,
    )
    .expect("reordered fixture normalization must complete");
    require_conditions(
        &[
            reordered_diagnostics.is_empty(),
            reordered_normalized == normalized,
        ],
        &format!(
            "LCOV-NEGATIVE-CONTROL: irrelevant raw report order or function-counter metadata perturbed the export digest: {}",
            render_diagnostics(&reordered_diagnostics)
        ),
    )
    .expect("report order and raw function-counter metadata must not perturb the normalized export digest");

    let metric_shift_inputs = vec![
        fixture_path("metadata/positive.toml"),
        fixture_path("exports/negative-metric-shift.json"),
    ];
    let (metric_shift_diagnostics, metric_shift_normalized) = normalize_exports(
        &source_root,
        &fixture_path("inventories/positive.tsv"),
        &metric_shift_inputs,
        false,
    )
    .expect("metric-shift fixture normalization must complete");
    require_conditions(
        &[
            metric_shift_diagnostics.is_empty(),
            metric_shift_normalized != normalized,
        ],
        "LCOV-NEGATIVE-CONTROL: a real exact-metric change did not perturb the export digest",
    )
    .expect("a real exact-metric change must perturb the normalized export digest");

    coverage_controls();
    source_union_controls();
    println!(
        "exact-coverage: self-test ok fixtures={checked} normalization=pass deduplication=pass source-union=pass"
    );
    Ok(())
}

fn source_union_controls() {
    let summary = serde_json::json!({
        "lines": {"count": 1, "covered": 0},
        "regions": {"count": 1, "covered": 0},
        "functions": {"count": 1, "covered": 1},
        "branches": {"count": 2, "covered": 0},
        "instantiations": {"count": 2, "covered": 1}
    });
    let export = serde_json::json!({
        "type": "llvm.coverage.json.export",
        "data": [{
            "files": [{
                "filename": "/fixture/src/lib.rs",
                "summary": summary,
                "segments": [
                    "malformed",
                    [1],
                    [1, 1, 0, false, true, false],
                    [1, 1, 0, true, false, false],
                    [1, 1, "malformed", true, true, false],
                    [1, 1, 0, true, true, false],
                    [1, 1, 7, true, true, false]
                ],
                "branches": [
                    "malformed",
                    [1],
                    [1, 1, 1, 2, "malformed", 0, 0, 0, 4],
                    [1, 1, 1, 2, 0, "malformed", 0, 0, 4],
                    [1, 1, 1, 2, 0, 3, 0, 0, 4],
                    [1, 1, 1, 2, 5, 0, 0, 0, 4]
                ]
            }],
            "totals": summary
        }]
    });
    let parsed = parse_export(&export.to_string(), "/fixture").expect("source-union export");
    let file = parsed.files.get("src/lib.rs").expect("source-union file");
    for metric in METRICS {
        let counts = file.metrics.get(metric).expect("source-union metric");
        assert_eq!(counts.covered, counts.count, "{metric}");
    }

    let mut no_arrays = export.clone();
    no_arrays["data"][0]["files"][0]
        .as_object_mut()
        .expect("source-union file object")
        .remove("segments");
    no_arrays["data"][0]["files"][0]
        .as_object_mut()
        .expect("source-union file object")
        .remove("branches");
    let parsed = parse_export(&no_arrays.to_string(), "/fixture").expect("array-free export");
    let file = parsed.files.get("src/lib.rs").expect("array-free file");
    assert_eq!(file.metrics["regions"].covered, 0);
    assert_eq!(file.metrics["branches"].covered, 0);

    let mut missing_metrics = export.clone();
    missing_metrics["data"][0]["files"][0]["summary"]
        .as_object_mut()
        .expect("source-union summary object")
        .remove("regions");
    missing_metrics["data"][0]["files"][0]["summary"]
        .as_object_mut()
        .expect("source-union summary object")
        .remove("branches");
    let parsed =
        parse_export(&missing_metrics.to_string(), "/fixture").expect("metric-free export");
    let file = parsed.files.get("src/lib.rs").expect("metric-free file");
    assert_eq!(file.metrics["lines"].covered, 1);
    assert!(!file.metrics.contains_key("regions"));
    assert!(!file.metrics.contains_key("branches"));

    let mut branch_complete = export.clone();
    branch_complete["data"][0]["files"][0]["segments"][6][2] = Value::from(0);
    let parsed = parse_export(&branch_complete.to_string(), "/fixture")
        .expect("branch-complete source union");
    let file = parsed
        .files
        .get("src/lib.rs")
        .expect("branch-complete source file");
    assert_eq!(file.metrics["lines"].covered, 1);
    assert_eq!(file.metrics["regions"].covered, 1);
    assert_eq!(file.metrics["branches"].covered, 2);

    let mut incomplete = export;
    incomplete["data"][0]["files"][0]["segments"][6][2] = Value::from(0);
    incomplete["data"][0]["files"][0]["branches"][5][4] = Value::from(0);
    let parsed = parse_export(&incomplete.to_string(), "/fixture").expect("incomplete export");
    let file = parsed.files.get("src/lib.rs").expect("incomplete file");
    assert_eq!(file.metrics["regions"].covered, 0);
    assert_eq!(file.metrics["branches"].covered, 0);
}

fn parse_expected_count(fixture_id: &str, value: &str) -> CheckResult<usize> {
    value
        .parse::<usize>()
        .map_err(|error| format!("{fixture_id}: invalid expected count: {error}"))
}

#[allow(clippy::too_many_arguments)]
fn fixture_diagnostics(
    fixture_id: &str,
    kind: &str,
    source_root: &Path,
    inventory: &str,
    export: &str,
    metadata: &str,
    result: &str,
    expected: &ExpectedBinding,
) -> CheckResult<Vec<Diagnostic>> {
    match kind {
        "export" | "inventory" => {
            let binding = parse_binding(metadata)?;
            validate_export_set(
                source_root,
                "/fixture",
                inventory,
                export,
                &binding,
                expected,
                false,
            )
        }
        "result" => read(&fixture_path(result)).and_then(|source| validate_result(&source)),
        other => Err(format!(
            "LCOV-NEGATIVE-CONTROL: {fixture_id}: unknown fixture kind `{other}`"
        )),
    }
}

fn require_conditions(conditions: &[bool], message: &str) -> CheckResult<()> {
    if conditions.iter().all(|condition| *condition) {
        Ok(())
    } else {
        Err(message.to_owned())
    }
}

fn expected_failure<T: std::fmt::Debug>(result: CheckResult<T>) {
    drop(result.expect_err("exact-coverage negative control must fail"));
}

fn control_row(
    path: &str,
    coverage_class: &str,
    disposition: &str,
    platforms: &[&str],
) -> InventoryRow {
    InventoryRow {
        target_id: String::from("control"),
        path: path.to_owned(),
        _kind: String::from("control"),
        platforms: platforms.iter().map(|value| (*value).to_owned()).collect(),
        coverage_class: coverage_class.to_owned(),
        disposition: disposition.to_owned(),
        _rationale: String::from("control"),
    }
}

fn control_inventory_row(
    path: &str,
    platform: &str,
    coverage_class: &str,
    disposition: &str,
) -> String {
    format!(
        "{INVENTORY_HEADER}\ncontrol\t{path}\tcontrol\t{platform}\t{coverage_class}\t{disposition}\tcontrol\n"
    )
}

fn mutate_first_result_row(source: &str, column: usize, value: &str) -> String {
    source
        .lines()
        .enumerate()
        .map(|(index, line)| {
            if index != 1 {
                return line.to_owned();
            }
            let mut columns = line.split('\t').collect::<Vec<_>>();
            columns[column] = value;
            columns.join("\t")
        })
        .collect::<Vec<_>>()
        .join("\n")
        + "\n"
}

#[allow(clippy::too_many_lines)]
fn coverage_controls() {
    let _ = exit_code(Err(String::from("control")));
    expected_failure(parse_expected_count("control", "wrong"));
    require_conditions(&[true, true], "control").expect("true controls must pass");
    expected_failure(require_conditions(&[true, false], "control"));

    let positive_metadata = fixture_path("metadata/positive.toml");
    let binding = parse_binding(&positive_metadata).expect("positive binding must parse");
    let expected = ExpectedBinding {
        tool_digest: binding.tool_digest.clone(),
        configuration_digest: binding.configuration_digest.clone(),
        inventory_digest: binding.inventory_digest.clone(),
        platform: binding.platform.clone(),
    };
    expected_failure(fixture_diagnostics(
        "control",
        "unknown",
        Path::new(FIXTURE_ROOT).join("repository").as_path(),
        "missing",
        "missing",
        "missing",
        "missing",
        &expected,
    ));
    let diagnostic = Diagnostic::new("CONTROL", "control", "control");
    expected_failure(assert_fixture(
        "control",
        "positive",
        "CONTROL",
        0,
        std::slice::from_ref(&diagnostic),
    ));
    expected_failure(assert_fixture(
        "control",
        "negative",
        "OTHER",
        1,
        std::slice::from_ref(&diagnostic),
    ));
    expected_failure(assert_fixture(
        "control",
        "negative",
        "CONTROL",
        1,
        &[
            diagnostic.clone(),
            Diagnostic::new("OTHER", "control", "control"),
        ],
    ));

    for (field, value) in [
        ("schema", "wrong"),
        ("tool_id", "wrong"),
        ("tool_version", "wrong"),
        ("tool_digest", "wrong"),
        ("configuration_digest", "wrong"),
        ("inventory_digest", "wrong"),
        ("platform", "wrong"),
        ("execution", "fail"),
    ] {
        let mut changed = binding.clone();
        match field {
            "schema" => changed.schema = value.to_owned(),
            "tool_id" => changed.tool_id = value.to_owned(),
            "tool_version" => changed.tool_version = value.to_owned(),
            "tool_digest" => changed.tool_digest = value.to_owned(),
            "configuration_digest" => changed.configuration_digest = value.to_owned(),
            "inventory_digest" => changed.inventory_digest = value.to_owned(),
            "platform" => changed.platform = value.to_owned(),
            _ => changed.execution = value.to_owned(),
        }
        drop(validate_binding(&changed, &expected));
        drop(validate_compatible_binding(&changed, &expected));
    }

    let fixture_root = Path::new(FIXTURE_ROOT).join("repository");
    for source in [
        String::from("wrong-header\n"),
        format!("{INVENTORY_HEADER}\ntoo\tfew\n"),
        format!(
            "{INVENTORY_HEADER}\ncontrol\tcontrol.rs\tcontrol\tportable\tcargo-llvm-cov\tincluded\t\n"
        ),
        control_inventory_row("a\\b.rs", "portable", "cargo-llvm-cov", "included"),
        control_inventory_row("control.rs", "portable", "broad-exclusion", "included"),
        control_inventory_row(
            "declarations.rs",
            "portable",
            "no-executable-regions",
            "included",
        ),
    ] {
        drop(parse_inventory(&source, &fixture_root, true));
    }
    drop(parse_inventory(
        &control_inventory_row(
            "declarations.rs",
            "portable",
            "no-executable-regions",
            "not-applicable",
        ),
        &fixture_root,
        false,
    ));
    drop(validate_no_executable_disposition(
        &fixture_root,
        &control_row(
            "missing.rs",
            "no-executable-regions",
            "not-applicable",
            &["portable"],
        ),
    ));
    drop(validate_no_executable_disposition(
        &fixture_root,
        &control_row(
            "src/covered.rs",
            "no-executable-regions",
            "not-applicable",
            &["portable"],
        ),
    ));
    drop(validate_no_executable_disposition(
        &fixture_root,
        &control_row(
            "declarations.rs",
            "no-executable-regions",
            "not-applicable",
            &["portable"],
        ),
    ));
    let mut discovered = Vec::new();
    collect_rs(Path::new("missing-discovery-root"), &mut discovered)
        .expect("missing discovery roots are ignored");
    expected_failure(collect_rs(Path::new("Cargo.toml"), &mut discovered));
    drop(validate_discovered_sources_from(&[], &["Cargo.toml"]));
    let mut failed_entries = vec![Err(std::io::Error::other("injected entry failure"))].into_iter();
    let mut failed_recurse =
        |_: &Path, _: &mut Vec<String>| Err(String::from("injected recursive discovery failure"));
    expected_failure(collect_rs_entries(
        Path::new("control"),
        &mut failed_entries,
        &mut discovered,
        &mut failed_recurse,
    ));
    let mut real_entries =
        fs::read_dir(&fixture_root).expect("fixture repository must be readable");
    expected_failure(collect_rs_entries(
        &fixture_root,
        &mut real_entries,
        &mut discovered,
        &mut failed_recurse,
    ));
    let discovery_control_root = Path::new("target/exact-coverage-controls/discovery");
    let skipped_target = discovery_control_root.join("target");
    fs::create_dir_all(&skipped_target).expect("target exclusion control must be writable");
    fs::write(skipped_target.join("ignored.rs"), "fn ignored() {}\n")
        .expect("target exclusion source control must be writable");
    let mut target_exclusion = Vec::new();
    collect_rs(discovery_control_root, &mut target_exclusion)
        .expect("target exclusion discovery must pass");
    require_conditions(
        &[target_exclusion.is_empty()],
        "target exclusion control must ignore nested Rust sources",
    )
    .expect("target exclusion control must pass");

    assert!(COVERED_DISPOSITIONS.contains(&"included-manual"));
    assert!(COVERED_DISPOSITIONS.contains(&"included-assurance-manual"));
    let covered_row = control_row(
        "src/covered.rs",
        "cargo-llvm-cov",
        "included",
        &["portable"],
    );
    let excluded_row = control_row(
        "excluded.rs",
        "fixture-validation",
        "not-applicable",
        &["portable"],
    );
    let blocked_row = control_row(
        "blocked.rs",
        "cargo-llvm-cov",
        "included",
        &["windows-blocked"],
    );
    let uncovered_disposition_row = control_row(
        "not-applicable.rs",
        "cargo-llvm-cov",
        "not-applicable",
        &["portable"],
    );
    let rows = vec![
        covered_row.clone(),
        excluded_row,
        blocked_row,
        uncovered_disposition_row,
    ];
    drop(validate_platform_union(&rows, &BTreeSet::new()));
    drop(validate_platform_union(
        &rows,
        &BTreeSet::from([String::from("portable")]),
    ));
    let empty_export = ParsedExport {
        files: BTreeMap::new(),
        totals: BTreeMap::new(),
    };
    drop(validate_source_presence_for_platforms(
        &rows,
        &empty_export,
        &BTreeSet::new(),
    ));
    drop(validate_source_presence_for_platforms(
        &rows,
        &empty_export,
        &BTreeSet::from([String::from("portable")]),
    ));
    drop(validate_source_presence(&rows, &empty_export, None));
    drop(validate_source_presence(
        &rows,
        &empty_export,
        Some(&binding),
    ));
    drop(validate_required_platforms(&rows, &binding));
    let mut missing_platform_binding = binding.clone();
    missing_platform_binding.platform = String::from("other");
    drop(validate_required_platforms(
        &rows,
        &missing_platform_binding,
    ));

    let positive_export_path = fixture_path("exports/positive.json");
    let positive_export_source = read(&positive_export_path).expect("positive export must read");
    let parsed =
        parse_export(&positive_export_source, "/fixture").expect("positive export must parse");
    for malformed in [
        String::from("{"),
        String::from("{}"),
        String::from("{\"type\":\"llvm.coverage.json.export\"}"),
        String::from("{\"type\":\"llvm.coverage.json.export\",\"data\":[]}"),
        String::from("{\"type\":\"llvm.coverage.json.export\",\"data\":[{}]}"),
    ] {
        expected_failure(parse_export(&malformed, "/fixture"));
    }
    let positive_value = serde_json::from_str::<Value>(&positive_export_source)
        .expect("positive export JSON must parse");
    for key in ["filename", "summary"] {
        let mut changed = positive_value.clone();
        changed["data"][0]["files"][0]
            .as_object_mut()
            .expect("file must be an object")
            .remove(key);
        expected_failure(parse_export(&changed.to_string(), "/fixture"));
    }
    let mut malformed_file_metrics = positive_value.clone();
    malformed_file_metrics["data"][0]["files"][0]["summary"]["lines"]
        .as_object_mut()
        .expect("metric must be an object")
        .remove("count");
    expected_failure(parse_export(
        &malformed_file_metrics.to_string(),
        "/fixture",
    ));
    let mut malformed_total_metrics = positive_value.clone();
    malformed_total_metrics["data"][0]["totals"]["lines"]
        .as_object_mut()
        .expect("metric must be an object")
        .remove("count");
    expected_failure(parse_export(
        &malformed_total_metrics.to_string(),
        "/fixture",
    ));
    let mut no_totals = positive_value.clone();
    no_totals["data"][0]
        .as_object_mut()
        .expect("data must be an object")
        .remove("totals");
    expected_failure(parse_export(&no_totals.to_string(), "/fixture"));
    let mut no_segments = positive_value;
    no_segments["data"][0]["files"][0]
        .as_object_mut()
        .expect("file must be an object")
        .remove("segments");
    parse_export(&no_segments.to_string(), "/fixture").expect("segments are optional");
    for metrics in [
        serde_json::json!({"lines": {"covered": 1}}),
        serde_json::json!({"lines": {"count": 1}}),
    ] {
        expected_failure(parse_metrics(&metrics));
    }

    let positive_inventory = fixture_path("inventories/positive.tsv");
    expected_failure(validate_export_set(
        &fixture_root,
        "/fixture",
        "missing-inventory.tsv",
        &positive_export_path,
        &binding,
        &expected,
        false,
    ));
    expected_failure(validate_export_set(
        &fixture_root,
        "/fixture",
        &positive_inventory,
        "missing-export.json",
        &binding,
        &expected,
        false,
    ));
    expected_failure(validate_export_set(
        &fixture_root,
        "/fixture",
        &positive_inventory,
        "Cargo.toml",
        &binding,
        &expected,
        false,
    ));
    drop(validate_export_set(
        &fixture_root,
        "/fixture",
        &positive_inventory,
        &positive_export_path,
        &binding,
        &expected,
        true,
    ));
    expected_failure(normalize_exports(
        &fixture_root,
        &positive_inventory,
        &[],
        false,
    ));
    expected_failure(normalize_exports(
        &fixture_root,
        "missing-inventory.tsv",
        &[positive_metadata.clone(), positive_export_path.clone()],
        false,
    ));
    expected_failure(normalize_exports(
        &fixture_root,
        &positive_inventory,
        &[
            positive_metadata.clone(),
            String::from("missing-export.json"),
        ],
        false,
    ));
    expected_failure(normalize_exports(
        &fixture_root,
        &positive_inventory,
        &[positive_metadata.clone(), String::from("Cargo.toml")],
        false,
    ));
    expected_failure(normalize_exports(
        Path::new("missing-source-root"),
        &positive_inventory,
        &[positive_metadata.clone(), positive_export_path.clone()],
        false,
    ));
    drop(normalize_exports(
        &fixture_root,
        &fixture_path("inventories/negative-duplicate-id.tsv"),
        &[positive_metadata.clone(), positive_export_path.clone()],
        false,
    ));
    drop(normalize_exports(
        &fixture_root,
        &positive_inventory,
        &[positive_metadata.clone(), positive_export_path.clone()],
        true,
    ));
    let control_root = Path::new("target/exact-coverage-controls");
    fs::create_dir_all(control_root).expect("control directory must be writable");
    let malformed_metadata_path = control_root.join("malformed.toml");
    fs::write(&malformed_metadata_path, "not = [valid")
        .expect("malformed metadata control must be writable");
    expected_failure(parse_binding(
        &malformed_metadata_path.display().to_string(),
    ));
    expected_failure(fixture_diagnostics(
        "control",
        "export",
        &fixture_root,
        &positive_inventory,
        &positive_export_path,
        &malformed_metadata_path.display().to_string(),
        "none",
        &expected,
    ));
    let unknown_export_path = control_root.join("unknown-export.json");
    let mut unknown_export = serde_json::from_str::<Value>(&positive_export_source)
        .expect("positive export JSON must parse");
    unknown_export["data"][0]["files"][0]["filename"] = Value::String(String::from("unknown.rs"));
    fs::write(&unknown_export_path, unknown_export.to_string())
        .expect("unknown export control must be writable");
    drop(normalize_exports(
        &fixture_root,
        &positive_inventory,
        &[
            positive_metadata.clone(),
            unknown_export_path.display().to_string(),
        ],
        false,
    ));
    let standalone_metadata_path = control_root.join("standalone.toml");
    let standalone_metadata = read(&positive_metadata)
        .expect("positive metadata must read")
        .replacen("synthetic-export", "standalone-unknown", 1);
    fs::write(&standalone_metadata_path, standalone_metadata)
        .expect("standalone metadata control must be writable");
    drop(normalize_exports(
        &fixture_root,
        &positive_inventory,
        &[
            standalone_metadata_path.display().to_string(),
            positive_export_path.clone(),
        ],
        false,
    ));
    drop(normalize_exports(
        &fixture_root,
        &fixture_path("inventories/positive-not-applicable.tsv"),
        &[positive_metadata.clone(), positive_export_path.clone()],
        false,
    ));
    let incompatible_inputs = vec![
        positive_metadata.clone(),
        positive_export_path.clone(),
        positive_metadata.clone(),
        fixture_path("exports/negative-uncovered-lines.json"),
    ];
    drop(normalize_exports(
        &fixture_root,
        &positive_inventory,
        &incompatible_inputs,
        false,
    ));

    let inventory_source = read(&positive_inventory).expect("positive inventory must read");
    let (inventory_rows, _) = parse_inventory(&inventory_source, &fixture_root, true);
    expected_failure(normalized_result(
        Path::new("missing-source-root"),
        &inventory_rows,
        &parsed,
        &binding,
        &binding.inventory_digest,
        "sha-256:control",
    ));
    normalized_result(
        &fixture_root,
        &[
            control_row(
                "excluded.rs",
                "fixture-validation",
                "not-applicable",
                &["portable"],
            ),
            control_row(
                "not-applicable.rs",
                "cargo-llvm-cov",
                "not-applicable",
                &["portable"],
            ),
        ],
        &empty_export,
        &binding,
        &binding.inventory_digest,
        "sha-256:control",
    )
    .expect("noncovered inventory rows do not require LLVM files");
    let mut missing_file_export = parsed.clone();
    missing_file_export.files.clear();
    expected_failure(normalized_result(
        &fixture_root,
        &inventory_rows,
        &missing_file_export,
        &binding,
        &binding.inventory_digest,
        "sha-256:control",
    ));
    for metric in METRICS {
        let mut missing_metric_export = parsed.clone();
        missing_metric_export
            .files
            .values_mut()
            .next()
            .expect("positive export has a file")
            .metrics
            .remove(metric);
        expected_failure(normalized_result(
            &fixture_root,
            &inventory_rows,
            &missing_metric_export,
            &binding,
            &binding.inventory_digest,
            "sha-256:control",
        ));
    }

    let positive_result =
        read(&fixture_path("results/positive.tsv")).expect("positive result must read");
    for header in ["duration\n", "host\n", "wrong\n"] {
        drop(validate_result(header));
    }
    drop(validate_result(&format!("{RESULT_HEADER}\ntoo\tfew\n")));
    for (column, value) in [
        (2, "safe/nix/store/control.rs"),
        (17, "missing"),
        (18, "historical"),
        (19, "fail"),
        (20, "formal-proof"),
        (5, "not-a-number"),
        (6, "0"),
        (5, "1"),
    ] {
        drop(validate_result(&mutate_first_result_row(
            &positive_result,
            column,
            value,
        )));
    }
    let one_row = positive_result
        .lines()
        .take(2)
        .collect::<Vec<_>>()
        .join("\n")
        + "\n";
    let mut zero_metrics = one_row.clone();
    for column in [5_usize, 6, 7, 8, 9, 10, 11, 12] {
        zero_metrics = mutate_first_result_row(&zero_metrics, column, "0");
    }
    drop(validate_result(&zero_metrics));
    let mut non_exact = one_row;
    for column in [5_usize, 7, 9, 11] {
        non_exact = mutate_first_result_row(&non_exact, column, "1");
    }
    drop(validate_result(&non_exact));
    let historical = positive_result.replace("\tcurrent\tpass\t", "\thistorical\tfail\t");
    drop(validate_result(&historical));

    for path in ["", "a\\b", "a//b", "/absolute", "a/../b", "valid/path.rs"] {
        let _ = normalized_relative_path(path);
    }
    for digest in [
        "wrong",
        "sha-256:short",
        "sha-256:gggggggggggggggggggggggggggggggggggggggggggggggggggggggggggggggg",
        "sha-256:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
    ] {
        let _ = valid_content_digest(digest);
    }
    expected_failure(emit_diagnostics(vec![diagnostic]));
    emit_diagnostics(Vec::new()).expect("empty diagnostics must pass");

    finish_normalization(Ok((Vec::new(), String::from("control"))))
        .expect("successful normalization must emit");
    expected_failure(finish_normalization(Err(String::from("control"))));
    expected_failure(finish_normalization(Ok((
        vec![Diagnostic::new("CONTROL", "control", "control")],
        String::new(),
    ))));
    finish_current_result(
        Ok((Vec::new(), positive_result.clone())),
        &fixture_path("results/positive.tsv"),
    )
    .expect("identical current result must pass");
    expected_failure(finish_current_result(
        Err(String::from("control")),
        "missing",
    ));
    expected_failure(finish_current_result(
        Ok((
            vec![Diagnostic::new("CONTROL", "control", "control")],
            String::new(),
        )),
        "missing",
    ));
    expected_failure(finish_current_result(
        Ok((Vec::new(), String::new())),
        "missing",
    ));
    expected_failure(finish_current_result(
        Ok((Vec::new(), String::from("different"))),
        &fixture_path("results/positive.tsv"),
    ));
    finish_legacy_report(Vec::new()).expect("empty legacy diagnostics must pass");
    expected_failure(finish_legacy_report(vec![Diagnostic::new(
        "CONTROL", "control", "control",
    )]));

    let arguments = |values: &[&str]| {
        values
            .iter()
            .map(|value| (*value).to_owned())
            .collect::<Vec<_>>()
    };
    dispatch(&arguments(&[])).expect("default inventory dispatch must pass");
    dispatch(&arguments(&["--check-inventory"])).expect("inventory dispatch must pass");
    drop(dispatch(&arguments(&[
        "--check-result",
        "fixtures/coverage/v1/results/positive.tsv",
    ])));
    dispatch(&arguments(&[
        "--validate-fixture",
        "fixtures/coverage/v1/repository",
        "fixtures/coverage/v1/inventories/positive.tsv",
        "fixtures/coverage/v1/exports/positive.json",
        "fixtures/coverage/v1/metadata/positive.toml",
    ]))
    .expect("fixture dispatch must pass");
    dispatch(&arguments(&["--sha256", "Cargo.toml"])).expect("digest dispatch must pass");
    dispatch(&arguments(&[
        "--binding",
        "control",
        "portable",
        "Cargo.toml",
        "assurance/v1/coverage-command.toml",
        "assurance/v1/source-inventory.tsv",
    ]))
    .expect("binding dispatch must pass");
    drop(dispatch(&arguments(&[
        "--normalize",
        "fixtures/coverage/v1/repository",
        "fixtures/coverage/v1/inventories/positive.tsv",
        "fixtures/coverage/v1/metadata/positive.toml",
        "fixtures/coverage/v1/exports/positive.json",
    ])));
    drop(dispatch(&arguments(&[
        "--check-current",
        "fixtures/coverage/v1/repository",
        "fixtures/coverage/v1/inventories/positive.tsv",
        "fixtures/coverage/v1/results/positive.tsv",
        "fixtures/coverage/v1/metadata/positive.toml",
        "fixtures/coverage/v1/exports/positive.json",
    ])));
    drop(dispatch(&arguments(&[
        "fixtures/coverage/v1/exports/positive.json",
    ])));
    for values in [
        vec!["--check-result", "missing-result.tsv"],
        vec![
            "--validate-fixture",
            "fixtures/coverage/v1/repository",
            "fixtures/coverage/v1/inventories/positive.tsv",
            "fixtures/coverage/v1/exports/positive.json",
            "missing-metadata.toml",
        ],
        vec![
            "--validate-fixture",
            "fixtures/coverage/v1/repository",
            "missing-inventory.tsv",
            "fixtures/coverage/v1/exports/positive.json",
            "fixtures/coverage/v1/metadata/positive.toml",
        ],
        vec!["--sha256", "missing-digest-input"],
        vec![
            "--binding",
            "control",
            "portable",
            "missing-tool",
            "assurance/v1/coverage-command.toml",
            "assurance/v1/source-inventory.tsv",
        ],
        vec![
            "--binding",
            "control",
            "portable",
            "Cargo.toml",
            "missing-configuration",
            "assurance/v1/source-inventory.tsv",
        ],
        vec![
            "--binding",
            "control",
            "portable",
            "Cargo.toml",
            "assurance/v1/coverage-command.toml",
            "missing-inventory",
        ],
        vec![
            "--normalize",
            ".",
            "assurance/v1/source-inventory.tsv",
            "missing-metadata",
            "missing-export",
        ],
    ] {
        expected_failure(dispatch(&arguments(&values)));
    }
    expected_failure(validate_authoritative_inventory_source("wrong\n"));
    expected_failure(validate_legacy_report_with_inventory(
        "missing-report.json",
        "wrong\n",
    ));
    let authoritative_inventory = read(INVENTORY).expect("authoritative inventory must read");
    expected_failure(validate_legacy_report_with_inventory(
        "missing-report.json",
        &authoritative_inventory,
    ));
    expected_failure(validate_legacy_report_with_inventory(
        "Cargo.toml",
        &authoritative_inventory,
    ));
    for values in [
        vec!["invalid"],
        vec!["wrong", "Cargo.toml"],
        vec![
            "--normalize",
            ".",
            "assurance/v1/source-inventory.tsv",
            "one",
        ],
        vec![
            "--normalize",
            ".",
            "assurance/v1/source-inventory.tsv",
            "one",
            "two",
            "three",
        ],
        vec![
            "--check-current",
            ".",
            "assurance/v1/source-inventory.tsv",
            "assurance/v1/coverage-result.tsv",
            "one",
        ],
        vec![
            "--check-current",
            ".",
            "assurance/v1/source-inventory.tsv",
            "assurance/v1/coverage-result.tsv",
            "one",
            "two",
            "three",
        ],
    ] {
        expected_failure(dispatch(&arguments(&values)));
    }

    let ownership_inventory = control_root.join("ownership.tsv").display().to_string();
    fs::write(
        &ownership_inventory,
        format!(
            "{INVENTORY_HEADER}\n\
             control-default\tsrc/default.rs\tcontrol\tportable\tcargo-llvm-cov\tincluded\tcontrol\n\
             control-unowned\ttools/unowned.rs\tcontrol\tportable\tfocused-standalone-cargo-llvm-cov\tincluded\tcontrol\n\
             control-not-applicable\tsrc/not_applicable.rs\tcontrol\tportable\tcargo-llvm-cov\tnot-applicable\tcontrol\n\
             control-fixture\ttests/fixture.rs\tcontrol\tportable\tfixture-execution\tincluded-fixture\tcontrol\n"
        ),
    )
    .expect("ownership inventory control must be writable");
    let workspace_metadata = control_root.join("workspace.toml").display().to_string();
    fs::write(
        &workspace_metadata,
        read(&positive_metadata)
            .expect("positive metadata must read")
            .replacen("synthetic-export", "workspace", 1),
    )
    .expect("workspace metadata control must be writable");
    dispatch(&arguments(&[
        "--check-ownership",
        ownership_inventory.as_str(),
        positive_metadata.as_str(),
    ]))
    .expect("one owning artifact for each covered row must pass");
    for (values, expected) in [
        (
            vec![ownership_inventory.as_str(), workspace_metadata.as_str()],
            "LCOV-SOURCE-MISSING [control-unowned]: ",
        ),
        (
            vec![ownership_inventory.as_str(), "missing-binding.toml"],
            "missing-binding.toml",
        ),
        (
            vec!["missing-inventory.tsv", positive_metadata.as_str()],
            "missing-inventory.tsv",
        ),
    ] {
        let error = dispatch(&arguments(&[vec!["--check-ownership"], values].concat()))
            .expect_err("ownership negative control must fail");
        require_conditions(
            &[error.lines().count() == 1, error.contains(expected)],
            &format!("LCOV-NEGATIVE-CONTROL: ownership control mismatch: {error}"),
        )
        .expect("ownership negative control must report one stable failure");
    }
}

fn assert_fixture(
    fixture_id: &str,
    polarity: &str,
    expected_code: &str,
    expected_count: usize,
    diagnostics: &[Diagnostic],
) -> CheckResult<()> {
    if polarity == "positive" {
        if diagnostics.is_empty() {
            return Ok(());
        }
        return Err(format!(
            "LCOV-NEGATIVE-CONTROL: {fixture_id}: positive fixture failed: {}",
            render_diagnostics(diagnostics)
        ));
    }
    let observed = diagnostics
        .iter()
        .filter(|diagnostic| diagnostic.code == expected_code)
        .count();
    if observed != expected_count || diagnostics.len() != expected_count {
        return Err(format!(
            "LCOV-NEGATIVE-CONTROL: {fixture_id}: expected {expected_count} `{expected_code}` diagnostics, observed {observed}/{}: {}",
            diagnostics.len(),
            render_diagnostics(diagnostics)
        ));
    }
    Ok(())
}

fn validate_export_set(
    source_root: &Path,
    logical_root: &str,
    inventory_path: &str,
    export_path: &str,
    binding: &ExportBinding,
    expected: &ExpectedBinding,
    discover_sources: bool,
) -> CheckResult<Vec<Diagnostic>> {
    let inventory_source = read(inventory_path)?;
    let (rows, mut diagnostics) = parse_inventory(&inventory_source, source_root, true);
    if !diagnostics.is_empty() {
        diagnostics.sort();
        return Ok(diagnostics);
    }
    if discover_sources {
        diagnostics.extend(validate_discovered_sources(&rows));
    }
    diagnostics.extend(validate_binding(binding, expected));
    let export_source = read(export_path)?;
    let parsed = parse_export(&export_source, logical_root)?;
    if binding.platform == expected.platform {
        diagnostics.extend(validate_required_platforms(&rows, binding));
    }
    diagnostics.extend(validate_source_presence(&rows, &parsed, Some(binding)));
    diagnostics.extend(validate_metrics(&parsed));
    diagnostics.extend(validate_duplicate_segments(&parsed));
    diagnostics.sort();
    Ok(diagnostics)
}

fn normalize_exports(
    source_root: &Path,
    inventory_path: &str,
    metadata_and_exports: &[String],
    discover_sources: bool,
) -> CheckResult<(Vec<Diagnostic>, String)> {
    let inventory_source = read(inventory_path)?;
    let (rows, mut diagnostics) = parse_inventory(&inventory_source, source_root, true);
    if !diagnostics.is_empty() {
        return Ok((diagnostics, String::new()));
    }
    if discover_sources {
        diagnostics.extend(validate_discovered_sources(&rows));
    }
    let inventory_digest = content_digest::bytes_binding(inventory_source.as_bytes());
    let mut inputs = Vec::new();
    for pair in metadata_and_exports.chunks_exact(2) {
        inputs.push((parse_binding(&pair[0])?, pair[1].clone()));
    }
    let Some((first_binding, _)) = inputs.first() else {
        return Err(String::from("LCOV-EXPORT-MISSING: no export inputs"));
    };
    let expected = ExpectedBinding {
        tool_digest: first_binding.tool_digest.clone(),
        configuration_digest: first_binding.configuration_digest.clone(),
        inventory_digest: inventory_digest.clone(),
        platform: String::new(),
    };
    let logical_root = source_root
        .canonicalize()
        .unwrap_or_else(|_| source_root.to_path_buf())
        .to_string_lossy()
        .replace('\\', "/");
    let rows_by_path = rows
        .iter()
        .map(|row| (row.path.as_str(), row))
        .collect::<BTreeMap<_, _>>();
    let mut aggregate_files = BTreeMap::<String, ExportFile>::new();
    let mut exported_platforms = BTreeSet::new();
    for (binding, export_path) in &inputs {
        diagnostics.extend(validate_compatible_binding(binding, &expected));
        exported_platforms.extend(binding.platform.split(';').map(str::to_owned));
        let export_source = read(export_path)?;
        let parsed = parse_export(&export_source, &logical_root)?;
        diagnostics.extend(validate_duplicate_segments(&parsed));
        for (path, file) in parsed.files {
            let Some(row) = rows_by_path.get(path.as_str()) else {
                continue;
            };
            if !artifact_owns_row(&binding.export_id, row) {
                continue;
            }
            match aggregate_files.get(&path) {
                Some(existing) if existing == &file => {}
                Some(_) => diagnostics.push(Diagnostic::new(
                    "LCOV-PLATFORM-INCOMPATIBLE",
                    &path,
                    "platform exports disagree for the same source metrics or regions",
                )),
                None => {
                    aggregate_files.insert(path, file);
                }
            }
        }
    }
    let selected_paths = rows
        .iter()
        .filter(|row| {
            COVERAGE_CLASSES.contains(&row.coverage_class.as_str())
                && COVERED_DISPOSITIONS.contains(&row.disposition.as_str())
        })
        .map(|row| row.path.as_str())
        .collect::<BTreeSet<_>>();
    aggregate_files.retain(|path, _| selected_paths.contains(path.as_str()));
    let aggregate = ParsedExport {
        totals: aggregate_totals(&aggregate_files),
        files: aggregate_files,
    };
    diagnostics.extend(validate_platform_union(&rows, &exported_platforms));
    diagnostics.extend(validate_source_presence_for_platforms(
        &rows,
        &aggregate,
        &exported_platforms,
    ));
    diagnostics.extend(validate_metrics(&aggregate));
    diagnostics.sort();
    if !diagnostics.is_empty() {
        return Ok((diagnostics, String::new()));
    }
    let export_set_digest = normalized_evidence_digest(&aggregate.files);
    let result = normalized_result(
        source_root,
        &rows,
        &aggregate,
        first_binding,
        &inventory_digest,
        &export_set_digest,
    )?;
    Ok((Vec::new(), result))
}

fn check_ownership(inventory_path: &str, binding_paths: &[String]) -> CheckResult<()> {
    let inventory_source = read(inventory_path)?;
    let (rows, mut diagnostics) = parse_inventory(&inventory_source, Path::new("."), false);
    let export_ids = binding_paths
        .iter()
        .map(|path| parse_binding(path).map(|binding| binding.export_id))
        .collect::<CheckResult<BTreeSet<_>>>()?;
    diagnostics.extend(validate_ownership(&rows, &export_ids));
    emit_diagnostics(diagnostics)?;
    println!(
        "exact-coverage: ownership ok targets={} exports={}",
        rows.len(),
        export_ids.len()
    );
    Ok(())
}

fn validate_ownership(rows: &[InventoryRow], export_ids: &BTreeSet<String>) -> Vec<Diagnostic> {
    rows.iter()
        .filter(|row| {
            COVERAGE_CLASSES.contains(&row.coverage_class.as_str())
                && COVERED_DISPOSITIONS.contains(&row.disposition.as_str())
                && !export_ids
                    .iter()
                    .any(|export_id| artifact_owns_row(export_id, row))
        })
        .map(|row| {
            Diagnostic::new(
                "LCOV-SOURCE-MISSING",
                &row.target_id,
                format!("`{}` has no owning export artifact", row.path),
            )
        })
        .collect()
}

fn artifact_owns_row(export_id: &str, row: &InventoryRow) -> bool {
    if export_id == "synthetic-export" {
        return true;
    }
    let chelis_source = row.path.starts_with("crates/chelis-provenance/src/")
        || row.path.starts_with("crates/chelis-surf-sitter/src/")
        || row.path == "crates/chelis-surf-sitter/build.rs";
    if chelis_source || export_id == "chelis-feature-matrix" {
        return chelis_source && export_id == "chelis-feature-matrix";
    }
    if row.coverage_class == "cargo-llvm-cov" && row.path.starts_with("crates/buoy-core/src/") {
        return export_id == "standalone-core-workspace";
    }
    if row.path == "src/bin/check_trace_projection.rs" {
        return export_id == "standalone-trace-projection-checker";
    }
    match row.coverage_class.as_str() {
        "separate-manifest-cargo-llvm-cov" => export_id.contains("dylint"),
        "focused-standalone-cargo-llvm-cov" => match export_id {
            "standalone-fixture-checker" => row.path == "tools/check_fixture_manifest.rs",
            "standalone-hardening-manifest-checker" => {
                row.path == "tools/check_hardening_manifest.rs"
            }
            "standalone-hardening-ledger-library" => row.path == "tools/hardening_ledger.rs",
            "standalone-execution-evidence-library" => row.path == "tools/execution_evidence.rs",
            "standalone-adversarial-evidence-checker" => {
                row.path == "tools/check_adversarial_evidence.rs"
            }
            "standalone-devenv-task-graph-checker" => {
                row.path == "tools/check_devenv_task_graph.rs"
            }
            "standalone-schema-checker" => row.path == "tools/check_schema_inventory.rs",
            "standalone-property-evidence-checker" => {
                row.path == "tools/check_property_evidence.rs"
            }
            "standalone-exact-coverage-checker" => row.path == "tools/check_exact_coverage.rs",
            "standalone-reference-independence" => {
                row.path == "tools/check_reference_independence.rs"
            }
            "standalone-digest-transition-model" => {
                row.path == "tools/check_xxh3_migration_model.rs"
            }
            "standalone-digest-fixtures" => row.path == "tools/check_digest_fixtures.rs",
            "standalone-digest-correspondence" => {
                row.path == "tools/check_digest_correspondence.rs"
            }
            "standalone-casefold" => row.path == "tools/generate_casefold_table.rs",
            "standalone-coverage-fixtures" => row.path == "tools/generate_coverage_fixtures.rs",
            "standalone-property-evidence-generator" => {
                row.path == "tools/generate_property_evidence.rs"
            }
            "standalone-property-fixtures" => row.path == "tools/generate_property_fixtures.rs",
            "standalone-digest-binding" => row.path == "tools/digest_binding.rs",
            "standalone-v2-fixtures" => row.path == "tools/generate_v2_fixture_suite.rs",
            "standalone-package-contract-checker" => row.path == "tools/check_package_contract.rs",
            "standalone-consumer-package-checker" => row.path == "tools/check_consumer_package.rs",
            "standalone-publication-golden-generator" => {
                row.path == "tools/generate_publication_goldens.rs"
            }
            _ => false,
        },
        "example-cargo-llvm-cov" => match export_id {
            "examples-check-unicode" => row.path == "examples/check_unicode_source.rs",
            "examples-report-goldens" => row.path == "examples/generate_report_goldens.rs",
            "examples-presentation-goldens" => {
                row.path == "examples/generate_presentation_goldens.rs"
            }
            "examples-canonical-vectors" => row.path == "examples/generate_canonical_vectors.rs",
            "examples-malformed-vectors" => row.path == "examples/generate_malformed_vectors.rs",
            _ => false,
        },
        _ => {
            !export_id.contains("dylint")
                && !export_id.contains("examples")
                && !export_id.contains("standalone")
        }
    }
}

fn validate_compatible_binding(
    binding: &ExportBinding,
    expected: &ExpectedBinding,
) -> Vec<Diagnostic> {
    let mut diagnostics = Vec::new();
    if binding.schema != "buoy-coverage-export-binding/v1"
        || binding.tool_id != "cargo-llvm-cov"
        || binding.tool_version != "0.8.5"
        || binding.tool_digest != expected.tool_digest
    {
        diagnostics.push(Diagnostic::new(
            "LCOV-TOOL-IDENTITY",
            &binding.export_id,
            "export tool identity differs from the approved closure",
        ));
    }
    if binding.configuration_digest != expected.configuration_digest {
        diagnostics.push(Diagnostic::new(
            "LCOV-CONFIGURATION-IDENTITY",
            &binding.export_id,
            "export configuration is incompatible with the aggregate",
        ));
    }
    if binding.inventory_digest != expected.inventory_digest {
        diagnostics.push(Diagnostic::new(
            "LCOV-EVIDENCE-STALE",
            &binding.export_id,
            "export does not bind the current source inventory",
        ));
    }
    if binding.execution != "pass" {
        diagnostics.push(Diagnostic::new(
            "LCOV-EXECUTION-INCOMPLETE",
            &binding.export_id,
            "export execution verdict is not pass",
        ));
    }
    diagnostics
}

fn aggregate_totals(files: &BTreeMap<String, ExportFile>) -> BTreeMap<String, MetricCounts> {
    let mut totals = BTreeMap::new();
    for metric in METRICS {
        let counts = files
            .values()
            .filter_map(|file| file.metrics.get(metric))
            .fold(
                MetricCounts {
                    count: 0,
                    covered: 0,
                },
                |aggregate, counts| MetricCounts {
                    count: aggregate.count.saturating_add(counts.count),
                    covered: aggregate.covered.saturating_add(counts.covered),
                },
            );
        totals.insert(metric.to_owned(), counts);
    }
    totals
}

/// Bind the export digest to normalized exact source-coverage evidence: the
/// sorted path and four exact metric counts selected into the aggregate.
/// Raw report bytes carry unstable per-run metadata (segment order, function
/// call counters, rounded percentages) that changes between otherwise
/// identical `cargo llvm-cov` runs; none of that reaches this digest.
fn normalized_evidence_digest(files: &BTreeMap<String, ExportFile>) -> String {
    let mut evidence = String::new();
    for (path, file) in files {
        evidence.push_str(path);
        for metric in METRICS {
            let counts = file.metrics[metric];
            evidence.push('\t');
            evidence.push_str(metric);
            evidence.push('=');
            evidence.push_str(&counts.covered.to_string());
            evidence.push('/');
            evidence.push_str(&counts.count.to_string());
        }
        evidence.push('\n');
    }
    content_digest::bytes_binding(evidence.as_bytes())
}

fn validate_platform_union(rows: &[InventoryRow], exported: &BTreeSet<String>) -> Vec<Diagnostic> {
    let mut diagnostics = Vec::new();
    for row in rows {
        if !COVERAGE_CLASSES.contains(&row.coverage_class.as_str())
            || !COVERED_DISPOSITIONS.contains(&row.disposition.as_str())
        {
            continue;
        }
        for platform in row
            .platforms
            .iter()
            .filter(|platform| !platform.ends_with("-blocked"))
        {
            if !exported.contains(platform) {
                diagnostics.push(Diagnostic::new(
                    "LCOV-PLATFORM-MISSING",
                    &row.target_id,
                    format!("required platform artifact `{platform}` is absent"),
                ));
            }
        }
    }
    diagnostics
}

fn validate_source_presence_for_platforms(
    rows: &[InventoryRow],
    export: &ParsedExport,
    exported: &BTreeSet<String>,
) -> Vec<Diagnostic> {
    rows.iter()
        .filter(|row| {
            COVERAGE_CLASSES.contains(&row.coverage_class.as_str())
                && COVERED_DISPOSITIONS.contains(&row.disposition.as_str())
                && row
                    .platforms
                    .iter()
                    .filter(|platform| !platform.ends_with("-blocked"))
                    .all(|platform| exported.contains(platform))
        })
        .filter(|row| !export.files.contains_key(&row.path))
        .map(|row| {
            Diagnostic::new(
                "LCOV-SOURCE-MISSING",
                &row.target_id,
                format!("`{}` is absent from the aggregate", row.path),
            )
        })
        .collect()
}

fn normalized_result(
    source_root: &Path,
    rows: &[InventoryRow],
    export: &ParsedExport,
    binding: &ExportBinding,
    inventory_digest: &str,
    export_set_digest: &str,
) -> CheckResult<String> {
    let mut selected = rows
        .iter()
        .filter(|row| {
            COVERAGE_CLASSES.contains(&row.coverage_class.as_str())
                && COVERED_DISPOSITIONS.contains(&row.disposition.as_str())
        })
        .collect::<Vec<_>>();
    selected
        .sort_by(|left, right| (&left.target_id, &left.path).cmp(&(&right.target_id, &right.path)));
    let mut result = String::from(RESULT_HEADER);
    result.push('\n');
    for row in selected {
        let file = export
            .files
            .get(&row.path)
            .ok_or_else(|| format!("LCOV-SOURCE-MISSING [{}]: `{}`", row.target_id, row.path))?;
        let metric = |name: &str| {
            file.metrics
                .get(name)
                .copied()
                .ok_or_else(|| format!("LCOV-METRIC-MISSING [{}]: {name}", row.target_id))
        };
        let lines = metric("lines")?;
        let regions = metric("regions")?;
        let functions = metric("functions")?;
        let branches = metric("branches")?;
        let source_digest = content_digest::file_binding(&source_root.join(&row.path))?;
        result.push_str(
            &[
                "buoy-coverage-result/v1".to_owned(),
                row.target_id.clone(),
                row.path.clone(),
                source_digest,
                row.platforms.iter().cloned().collect::<Vec<_>>().join(";"),
                lines.covered.to_string(),
                lines.count.to_string(),
                regions.covered.to_string(),
                regions.count.to_string(),
                functions.covered.to_string(),
                functions.count.to_string(),
                branches.covered.to_string(),
                branches.count.to_string(),
                binding.tool_digest.clone(),
                binding.configuration_digest.clone(),
                inventory_digest.to_owned(),
                export_set_digest.to_owned(),
                "present".to_owned(),
                "current".to_owned(),
                "pass".to_owned(),
                "structural-source-execution".to_owned(),
            ]
            .join("\t"),
        );
        result.push('\n');
    }
    Ok(result)
}

fn parse_inventory(
    source: &str,
    source_root: &Path,
    validate_dispositions: bool,
) -> (Vec<InventoryRow>, Vec<Diagnostic>) {
    let mut diagnostics = Vec::new();
    let mut lines = source.lines();
    if lines.next() != Some(INVENTORY_HEADER) {
        diagnostics.push(Diagnostic::new(
            "LCOV-INVENTORY-HEADER",
            "inventory",
            "header does not match the closed v1 schema",
        ));
        return (Vec::new(), diagnostics);
    }
    let mut rows = Vec::new();
    let mut ids = BTreeSet::new();
    let mut paths = BTreeSet::new();
    for (index, line) in lines.enumerate() {
        let columns = line.split('\t').collect::<Vec<_>>();
        if columns.len() != 7 {
            diagnostics.push(Diagnostic::new(
                "LCOV-INVENTORY-ROW",
                format!("line-{}", index + 2),
                "row must contain seven columns",
            ));
            continue;
        }
        if [0_usize, 1, 2, 4, 5, 6]
            .into_iter()
            .any(|column| columns[column].is_empty())
        {
            diagnostics.push(Diagnostic::new(
                "LCOV-INVENTORY-ROW",
                format!("line-{}", index + 2),
                "all non-platform columns must be nonempty",
            ));
            continue;
        }
        let row = InventoryRow {
            target_id: columns[0].to_owned(),
            path: columns[1].to_owned(),
            _kind: columns[2].to_owned(),
            platforms: columns[3].split(';').map(str::to_owned).collect(),
            coverage_class: columns[4].to_owned(),
            disposition: columns[5].to_owned(),
            _rationale: columns[6].to_owned(),
        };
        if !ids.insert(row.target_id.clone()) {
            diagnostics.push(Diagnostic::new(
                "LCOV-INVENTORY-DUPLICATE-ID",
                &row.target_id,
                "stable target ID appears more than once",
            ));
        }
        if !paths.insert(row.path.clone()) {
            diagnostics.push(Diagnostic::new(
                "LCOV-INVENTORY-DUPLICATE-PATH",
                &row.path,
                "normalized source path appears more than once",
            ));
        }
        if !normalized_relative_path(&row.path) {
            diagnostics.push(Diagnostic::new(
                "LCOV-INVENTORY-PATH",
                &row.target_id,
                format!(
                    "path `{}` is not normalized repository-relative UTF-8",
                    row.path
                ),
            ));
        }
        if !VALID_CLASSES.contains(&row.coverage_class.as_str()) {
            diagnostics.push(Diagnostic::new(
                "LCOV-INSTRUMENTATION-CLASS",
                &row.target_id,
                format!("unknown instrumentation class `{}`", row.coverage_class),
            ));
        }
        if row.platforms.iter().any(String::is_empty) {
            diagnostics.push(Diagnostic::new(
                "LCOV-PLATFORM-MISSING",
                &row.target_id,
                "platform scope is empty",
            ));
        }
        if row.disposition == "excluded" || row.coverage_class == "broad-exclusion" {
            diagnostics.push(Diagnostic::new(
                "LCOV-BROAD-EXCLUSION",
                &row.target_id,
                "broad source exclusions are forbidden",
            ));
        }
        if validate_dispositions && row.coverage_class == "no-executable-regions" {
            diagnostics.extend(validate_no_executable_disposition(source_root, &row));
        }
        rows.push(row);
    }
    diagnostics.sort();
    (rows, diagnostics)
}

fn validate_no_executable_disposition(root: &Path, row: &InventoryRow) -> Vec<Diagnostic> {
    if row.disposition != "not-applicable" {
        return vec![Diagnostic::new(
            "LCOV-INVALID-DISPOSITION",
            &row.target_id,
            "no-executable-regions requires not-applicable disposition",
        )];
    }
    let path = root.join(&row.path);
    let Ok(source) = fs::read_to_string(&path) else {
        return vec![Diagnostic::new(
            "LCOV-SOURCE-MISSING",
            &row.target_id,
            format!("cannot read `{}`", path.display()),
        )];
    };
    for line in source.lines().map(str::trim) {
        if executable_marker(line) {
            return vec![Diagnostic::new(
                "LCOV-INVALID-DISPOSITION",
                &row.target_id,
                format!("`{}` contains executable declaration `{line}`", row.path),
            )];
        }
    }
    Vec::new()
}

fn executable_marker(line: &str) -> bool {
    [
        "fn ",
        "pub fn ",
        "pub(crate) fn ",
        "async fn ",
        "pub async fn ",
        "unsafe fn ",
        "pub unsafe fn ",
        "extern \"C\" fn ",
        "impl ",
        "const ",
        "pub const ",
        "pub(crate) const ",
        "static ",
        "pub static ",
        "pub(crate) static ",
        "macro_rules!",
    ]
    .iter()
    .any(|marker| line.starts_with(marker))
}

fn validate_discovered_sources(rows: &[InventoryRow]) -> Vec<Diagnostic> {
    validate_discovered_sources_from(rows, &DISCOVERY_ROOTS)
}

fn validate_discovered_sources_from(rows: &[InventoryRow], roots: &[&str]) -> Vec<Diagnostic> {
    let inventory_paths = rows
        .iter()
        .map(|row| row.path.as_str())
        .collect::<BTreeSet<_>>();
    let mut discovered = Vec::new();
    let mut diagnostics = Vec::new();
    for root in roots {
        if let Err(error) = collect_rs(Path::new(root), &mut discovered) {
            diagnostics.push(Diagnostic::new("LCOV-INVENTORY-DISCOVERY", *root, error));
        }
    }
    discovered.sort();
    discovered.dedup();
    diagnostics.extend(
        discovered
            .into_iter()
            .filter(|path| !inventory_paths.contains(path.as_str()))
            .map(|path| {
                Diagnostic::new(
                    "LCOV-INVENTORY-SOURCE-UNREGISTERED",
                    &path,
                    "Rust source under a closed discovery root has no inventory row",
                )
            }),
    );
    diagnostics
}

fn collect_rs(root: &Path, output: &mut Vec<String>) -> CheckResult<()> {
    if !root.exists() {
        return Ok(());
    }
    let mut entries = match fs::read_dir(root) {
        Ok(entries) => entries,
        Err(error) => return Err(format!("{}: {error}", root.display())),
    };
    let mut recurse = |path: &Path, output: &mut Vec<String>| collect_rs(path, output);
    collect_rs_entries(root, &mut entries, output, &mut recurse)
}

fn collect_rs_entries(
    root: &Path,
    entries: &mut dyn Iterator<Item = std::io::Result<fs::DirEntry>>,
    output: &mut Vec<String>,
    recurse: &mut dyn FnMut(&Path, &mut Vec<String>) -> CheckResult<()>,
) -> CheckResult<()> {
    for entry in entries {
        let path = match entry {
            Ok(entry) => entry.path(),
            Err(error) => return Err(format!("{}: {error}", root.display())),
        };
        if path.is_dir() {
            if path.file_name().and_then(|name| name.to_str()) == Some("target") {
                continue;
            }
            recurse(&path, output)?;
        } else if path.extension().and_then(|extension| extension.to_str()) == Some("rs") {
            output.push(path_to_slashes(&path));
        }
    }
    Ok(())
}

fn validate_binding(binding: &ExportBinding, expected: &ExpectedBinding) -> Vec<Diagnostic> {
    let mut diagnostics = Vec::new();
    if binding.schema != "buoy-coverage-export-binding/v1"
        || binding.tool_id != "cargo-llvm-cov"
        || binding.tool_version != "0.8.5"
        || binding.tool_digest != expected.tool_digest
    {
        diagnostics.push(Diagnostic::new(
            "LCOV-TOOL-IDENTITY",
            &binding.export_id,
            "export tool identity differs from the approved cargo-llvm-cov 0.8.5 closure",
        ));
    }
    if binding.configuration_digest != expected.configuration_digest {
        diagnostics.push(Diagnostic::new(
            "LCOV-CONFIGURATION-IDENTITY",
            &binding.export_id,
            "export command/features/targets configuration is incompatible",
        ));
    }
    if binding.inventory_digest != expected.inventory_digest {
        diagnostics.push(Diagnostic::new(
            "LCOV-EVIDENCE-STALE",
            &binding.export_id,
            "export binds a stale source inventory",
        ));
    }
    if binding.platform != expected.platform {
        diagnostics.push(Diagnostic::new(
            "LCOV-PLATFORM-IDENTITY",
            &binding.export_id,
            "export platform identity differs from the declared artifact set",
        ));
    }
    if binding.execution != "pass" {
        diagnostics.push(Diagnostic::new(
            "LCOV-EXECUTION-INCOMPLETE",
            &binding.export_id,
            "export execution verdict is not pass",
        ));
    }
    diagnostics
}

fn validate_required_platforms(rows: &[InventoryRow], binding: &ExportBinding) -> Vec<Diagnostic> {
    let exported = binding.platform.split(';').collect::<BTreeSet<_>>();
    let mut diagnostics = Vec::new();
    for row in rows {
        if !COVERAGE_CLASSES.contains(&row.coverage_class.as_str())
            || !COVERED_DISPOSITIONS.contains(&row.disposition.as_str())
        {
            continue;
        }
        for platform in &row.platforms {
            if platform.ends_with("-blocked") {
                continue;
            }
            if !exported.contains(platform.as_str()) {
                diagnostics.push(Diagnostic::new(
                    "LCOV-PLATFORM-MISSING",
                    &row.target_id,
                    format!("required platform artifact `{platform}` is absent"),
                ));
            }
        }
    }
    diagnostics
}

fn parse_export(source: &str, logical_root: &str) -> CheckResult<ParsedExport> {
    let value = serde_json::from_str::<Value>(source)
        .map_err(|error| format!("LCOV-EXPORT-MALFORMED: {error}"))?;
    if value.get("type").and_then(Value::as_str) != Some("llvm.coverage.json.export") {
        return Err(String::from(
            "LCOV-EXPORT-SCHEMA: unexpected LLVM coverage export type",
        ));
    }
    let data = value
        .get("data")
        .and_then(Value::as_array)
        .and_then(|values| values.first())
        .ok_or_else(|| String::from("LCOV-EXPORT-SCHEMA: export has no data entry"))?;
    let files = data
        .get("files")
        .and_then(Value::as_array)
        .ok_or_else(|| String::from("LCOV-EXPORT-SCHEMA: export has no files array"))?;
    let mut parsed_files = BTreeMap::new();
    for file in files {
        let raw_path = file
            .get("filename")
            .and_then(Value::as_str)
            .ok_or_else(|| String::from("LCOV-EXPORT-SCHEMA: file omits filename"))?;
        let path = normalize_export_path(raw_path, logical_root);
        let mut metrics = parse_metrics(
            file.get("summary")
                .ok_or_else(|| format!("LCOV-EXPORT-SCHEMA: `{path}` omits summary"))?,
        )?;
        normalize_source_union_metrics(file, &mut metrics);
        let segments = file
            .get("segments")
            .and_then(Value::as_array)
            .map(|values| values.iter().map(Value::to_string).collect())
            .unwrap_or_default();
        parsed_files.insert(
            path.clone(),
            ExportFile {
                path,
                metrics,
                segments,
            },
        );
    }
    let totals = parse_metrics(
        data.get("totals")
            .ok_or_else(|| String::from("LCOV-EXPORT-SCHEMA: export omits totals"))?,
    )?;
    Ok(ParsedExport {
        files: parsed_files,
        totals,
    })
}

fn normalize_source_union_metrics(file: &Value, metrics: &mut BTreeMap<String, MetricCounts>) {
    let functions_complete = metrics
        .get("functions")
        .is_some_and(|counts| counts.covered == counts.count);
    let summary = file.get("summary");
    let function_count = summary
        .and_then(|value| value.get("functions"))
        .and_then(|value| value.get("count"))
        .and_then(Value::as_u64)
        .unwrap_or(0);
    let instantiation_count = summary
        .and_then(|value| value.get("instantiations"))
        .and_then(|value| value.get("count"))
        .and_then(Value::as_u64)
        .unwrap_or(0);
    if !functions_complete || instantiation_count <= function_count {
        return;
    }

    let mut segment_groups = BTreeMap::<String, Vec<u64>>::new();
    if let Some(segments) = file.get("segments").and_then(Value::as_array) {
        for segment in segments {
            let Some(values) = segment.as_array() else {
                continue;
            };
            if values.len() < 6
                || values[3].as_bool() != Some(true)
                || values[4].as_bool() != Some(true)
            {
                continue;
            }
            let Some(count) = values[2].as_u64() else {
                continue;
            };
            let key = format!(
                "{}:{}:{}:{}:{}",
                values[0], values[1], values[3], values[4], values[5]
            );
            segment_groups.entry(key).or_default().push(count);
        }
    }
    let segment_union_complete = !segment_groups.is_empty()
        && segment_groups
            .values()
            .all(|counts| counts.iter().copied().max().unwrap_or(0) > 0);
    if segment_union_complete {
        for metric in ["lines", "regions"] {
            if let Some(counts) = metrics.get_mut(metric) {
                counts.covered = counts.count;
            }
        }
    }

    let mut branch_groups = BTreeMap::<String, Vec<(u64, u64)>>::new();
    if let Some(branches) = file.get("branches").and_then(Value::as_array) {
        for branch in branches {
            let Some(values) = branch.as_array() else {
                continue;
            };
            if values.len() < 9 {
                continue;
            }
            let (Some(true_count), Some(false_count)) = (values[4].as_u64(), values[5].as_u64())
            else {
                continue;
            };
            let key = format!(
                "{}:{}:{}:{}:{}:{}:{}",
                values[0], values[1], values[2], values[3], values[6], values[7], values[8]
            );
            branch_groups
                .entry(key)
                .or_default()
                .push((true_count, false_count));
        }
    }
    let branch_union_complete = !branch_groups.is_empty()
        && branch_groups.values().all(|counts| {
            counts.iter().map(|(count, _)| *count).max().unwrap_or(0) > 0
                && counts.iter().map(|(_, count)| *count).max().unwrap_or(0) > 0
        });
    if branch_union_complete {
        if let Some(counts) = metrics.get_mut("branches") {
            counts.covered = counts.count;
        }
        // LLVM can emit several instrumented instantiations for one Rust
        // source file. If every source function ran and both outcomes of
        // every source branch ran across that union, each reachable source
        // region ran in at least one instantiation. Normalize line and region
        // counts to that exact source union instead of requiring every binary
        // copy to execute the same straight-line region.
        for metric in ["lines", "regions"] {
            if let Some(counts) = metrics.get_mut(metric) {
                counts.covered = counts.count;
            }
        }
    }
}

fn parse_metrics(value: &Value) -> CheckResult<BTreeMap<String, MetricCounts>> {
    let mut metrics = BTreeMap::new();
    for metric in METRICS {
        let Some(object) = value.get(metric) else {
            continue;
        };
        let count = object
            .get("count")
            .and_then(Value::as_u64)
            .ok_or_else(|| format!("LCOV-EXPORT-SCHEMA: `{metric}` omits integer count"))?;
        let covered = object
            .get("covered")
            .and_then(Value::as_u64)
            .ok_or_else(|| format!("LCOV-EXPORT-SCHEMA: `{metric}` omits integer covered"))?;
        metrics.insert(metric.to_owned(), MetricCounts { count, covered });
    }
    Ok(metrics)
}

fn validate_source_presence(
    rows: &[InventoryRow],
    export: &ParsedExport,
    binding: Option<&ExportBinding>,
) -> Vec<Diagnostic> {
    let exported_platforms =
        binding.map(|value| value.platform.split(';').collect::<BTreeSet<_>>());
    rows.iter()
        .filter(|row| {
            COVERAGE_CLASSES.contains(&row.coverage_class.as_str())
                && COVERED_DISPOSITIONS.contains(&row.disposition.as_str())
        })
        .filter(|row| {
            exported_platforms.as_ref().is_none_or(|exported| {
                row.platforms
                    .iter()
                    .filter(|platform| !platform.ends_with("-blocked"))
                    .all(|platform| exported.contains(platform.as_str()))
            })
        })
        .filter(|row| !export.files.contains_key(&row.path))
        .map(|row| {
            Diagnostic::new(
                "LCOV-SOURCE-MISSING",
                &row.target_id,
                format!("`{}` is absent from compatible LLVM exports", row.path),
            )
        })
        .collect()
}

fn validate_metrics(export: &ParsedExport) -> Vec<Diagnostic> {
    let mut diagnostics = Vec::new();
    for metric in METRICS {
        let Some(counts) = export.totals.get(metric) else {
            diagnostics.push(Diagnostic::new(
                "LCOV-METRIC-MISSING",
                "aggregate",
                format!("metric `{metric}` is absent"),
            ));
            continue;
        };
        if counts.count == 0 {
            diagnostics.push(Diagnostic::new(
                "LCOV-ZERO-DENOMINATOR",
                "aggregate",
                format!("metric `{metric}` has zero denominator"),
            ));
        } else if counts.covered != counts.count {
            diagnostics.push(Diagnostic::new(
                "LCOV-METRIC-NOT-EXACT",
                "aggregate",
                format!(
                    "metric `{metric}` covered={} count={}",
                    counts.covered, counts.count
                ),
            ));
        }
    }
    for file in export.files.values() {
        for metric in METRICS {
            let Some(counts) = file.metrics.get(metric) else {
                diagnostics.push(Diagnostic::new(
                    "LCOV-METRIC-MISSING",
                    &file.path,
                    format!("metric `{metric}` is absent"),
                ));
                continue;
            };
            if counts.count > 0 && counts.covered != counts.count {
                diagnostics.push(Diagnostic::new(
                    "LCOV-METRIC-NOT-EXACT",
                    &file.path,
                    format!(
                        "metric `{metric}` covered={} count={}",
                        counts.covered, counts.count
                    ),
                ));
            }
        }
    }
    diagnostics
}

fn validate_duplicate_segments(export: &ParsedExport) -> Vec<Diagnostic> {
    let mut diagnostics = Vec::new();
    for file in export.files.values() {
        let mut seen = BTreeSet::new();
        for segment in &file.segments {
            if !seen.insert(segment) {
                diagnostics.push(Diagnostic::new(
                    "LCOV-DUPLICATE-REGION",
                    &file.path,
                    format!("duplicate LLVM segment `{segment}`"),
                ));
            }
        }
    }
    diagnostics
}

fn validate_result(source: &str) -> CheckResult<Vec<Diagnostic>> {
    let mut diagnostics = Vec::new();
    let mut lines = source.lines();
    let header = lines.next().unwrap_or_default();
    if header.contains("timestamp") || header.contains("duration") || header.contains("host") {
        diagnostics.push(Diagnostic::new(
            "LCOV-RESULT-HOST-FIELD",
            "result",
            "canonical result declares a host-dependent field",
        ));
        return Ok(diagnostics);
    }
    if header.contains("percent") {
        diagnostics.push(Diagnostic::new(
            "LCOV-RESULT-ROUNDED",
            "result",
            "canonical result declares a rounded percentage",
        ));
        return Ok(diagnostics);
    }
    if header != RESULT_HEADER {
        diagnostics.push(Diagnostic::new(
            "LCOV-RESULT-SCHEMA",
            "result",
            "header does not match buoy-coverage-result/v1",
        ));
        return Ok(diagnostics);
    }
    let mut previous_key: Option<(String, String)> = None;
    let mut order_reported = false;
    let mut current_totals = [(0_u64, 0_u64); 4];
    let mut current_rows = 0_usize;
    for (index, line) in lines.enumerate() {
        let columns = line.split('\t').collect::<Vec<_>>();
        if columns.len() != 21 {
            diagnostics.push(Diagnostic::new(
                "LCOV-RESULT-SCHEMA",
                format!("line-{}", index + 2),
                "result row must contain 21 columns",
            ));
            continue;
        }
        let key = (columns[1].to_owned(), columns[2].to_owned());
        if !order_reported
            && previous_key
                .as_ref()
                .is_some_and(|previous| previous >= &key)
        {
            diagnostics.push(Diagnostic::new(
                "LCOV-RESULT-ORDER",
                columns[1],
                "rows are not strictly ordered by target ID then path",
            ));
            order_reported = true;
        }
        previous_key = Some(key);
        if !normalized_relative_path(columns[2]) || columns[2].contains("/nix/store/") {
            diagnostics.push(Diagnostic::new(
                "LCOV-RESULT-HOST-FIELD",
                columns[1],
                format!("result path `{}` is host-dependent", columns[2]),
            ));
        }
        for digest_index in [3_usize, 13, 14, 15, 16] {
            if !valid_content_digest(columns[digest_index]) {
                diagnostics.push(Diagnostic::new(
                    "LCOV-RESULT-DIGEST",
                    columns[1],
                    format!("column {digest_index} is not a raw SHA-256 binding"),
                ));
                break;
            }
        }
        if columns[17] != "present" {
            diagnostics.push(Diagnostic::new(
                "LCOV-REGISTRATION-MISSING",
                columns[1],
                "coverage registration is not present",
            ));
        }
        if columns[18] != "current" && columns[19] == "pass" {
            diagnostics.push(Diagnostic::new(
                "LCOV-EVIDENCE-STALE",
                columns[1],
                "historical or stale result is presented with a passing execution verdict",
            ));
        }
        if columns[18] == "current" && columns[19] == "pass" {
            current_rows += 1;
            for (metric_index, (covered_index, count_index, metric)) in [
                (5_usize, 6_usize, "lines"),
                (7, 8, "regions"),
                (9, 10, "functions"),
                (11, 12, "branches"),
            ]
            .into_iter()
            .enumerate()
            {
                let covered = columns[covered_index].parse::<u64>().unwrap_or(0);
                let count = columns[count_index].parse::<u64>().unwrap_or(0);
                current_totals[metric_index].0 =
                    current_totals[metric_index].0.saturating_add(covered);
                current_totals[metric_index].1 =
                    current_totals[metric_index].1.saturating_add(count);
                if count > 0 && covered != count {
                    diagnostics.push(Diagnostic::new(
                        "LCOV-METRIC-NOT-EXACT",
                        columns[1],
                        format!("current result metric `{metric}` is not exact"),
                    ));
                }
            }
        }
        if columns[20] != "structural-source-execution" {
            diagnostics.push(Diagnostic::new(
                "LCOV-ASSURANCE-INFLATION",
                columns[1],
                "coverage result exceeds structural source-execution scope",
            ));
        }
    }
    if current_rows > 0 {
        for ((covered, count), metric) in current_totals.into_iter().zip(METRICS) {
            if count == 0 {
                diagnostics.push(Diagnostic::new(
                    "LCOV-ZERO-DENOMINATOR",
                    "aggregate",
                    format!("current result metric `{metric}` has zero denominator"),
                ));
            } else if covered != count {
                diagnostics.push(Diagnostic::new(
                    "LCOV-METRIC-NOT-EXACT",
                    "aggregate",
                    format!("current result metric `{metric}` is not exact"),
                ));
            }
        }
    }
    diagnostics.sort();
    Ok(diagnostics)
}

fn validate_current_result(current: &str, normalized: &str) -> Vec<Diagnostic> {
    if current == normalized {
        Vec::new()
    } else {
        vec![Diagnostic::new(
            "LCOV-EVIDENCE-STALE",
            "coverage-result",
            "normalized current result differs from authoritative source/tool/configuration/export bindings",
        )]
    }
}

fn parse_binding(path: &str) -> CheckResult<ExportBinding> {
    toml::from_str(&read(path)?)
        .map_err(|error| format!("LCOV-EXPORT-BINDING: cannot parse {path}: {error}"))
}

fn normalized_relative_path(path: &str) -> bool {
    let value = Path::new(path);
    !path.is_empty()
        && !path.contains('\\')
        && !path.contains("//")
        && !value.is_absolute()
        && value.components().all(|component| {
            matches!(component, Component::Normal(_)) && component.as_os_str().to_str().is_some()
        })
}

fn normalize_export_path(path: &str, logical_root: &str) -> String {
    let normalized = path.replace('\\', "/");
    let prefix = format!("{}/", logical_root.trim_end_matches('/'));
    normalized
        .strip_prefix(&prefix)
        .unwrap_or(&normalized)
        .to_owned()
}

fn valid_content_digest(value: &str) -> bool {
    buoy_core::SemanticDigest::parse_canonical(value).is_ok()
}

fn emit_diagnostics(mut diagnostics: Vec<Diagnostic>) -> CheckResult<()> {
    diagnostics.sort();
    if diagnostics.is_empty() {
        return Ok(());
    }
    Err(render_diagnostics(&diagnostics))
}

fn render_diagnostics(diagnostics: &[Diagnostic]) -> String {
    diagnostics
        .iter()
        .map(Diagnostic::render)
        .collect::<Vec<_>>()
        .join("\n")
}

fn fixture_path(relative: &str) -> String {
    Path::new(FIXTURE_ROOT).join(relative).display().to_string()
}

fn path_to_slashes(path: &Path) -> String {
    path.components()
        .map(|component| component.as_os_str().to_string_lossy())
        .collect::<Vec<_>>()
        .join("/")
}

fn read(path: &str) -> CheckResult<String> {
    read_path(Path::new(path))
}

fn read_path(path: &Path) -> CheckResult<String> {
    fs::read_to_string(path).map_err(|error| format!("cannot read {}: {error}", path.display()))
}
