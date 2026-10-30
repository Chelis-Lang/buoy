//! Validate deterministic Hegel registration, execution results, freshness, and controls.

#![allow(
    clippy::assigning_clones,
    clippy::expect_used,
    clippy::too_many_lines,
    clippy::unnecessary_wraps
)]

use serde::Deserialize;
use std::collections::{BTreeMap, BTreeSet};
use std::env;
use std::fs;
use std::path::{Component, Path};

#[path = "digest_binding.rs"]
mod content_digest;

const INVENTORY: &str = "assurance/v1/property-inventory.tsv";
const RUNS: &str = "assurance/v1/property-runs.tsv";
const TOOL_BINDING: &str = "assurance/v1/hegel-tool-binding.toml";
const FIXTURE_ROOT: &str = "fixtures/property/v1";
const INVENTORY_HEADER: &str = "property_id\texpected_polarity\tproduction_target\tproduction_api\tproduction_source\treference_kind\treference_api\treference_source\ttest\ttest_source\tseed\tcases\tbackend\tderandomize\tdatabase\tphases\tgenerator_domain\tgenerator_limits\texplicit_edges\trejection_limit\tassertions\tsettings_digest\treplay_fixtures\tscope\tassurance_class\tstatus";
const RUN_HEADER: &str = "schema\tproperty_id\texpected_polarity\ttool_id\ttool_version\ttool_digest\ttoolchain_digest\tsettings_digest\tdomain_digest\tproperty_source_digest\tproduction_source_digest\treference_source_digest\treplay_digests\texecution\tdiagnostic\treplay_command\tidentity_binding\tassurance_class\tregistration\tfreshness";
const FIXTURE_MANIFEST_HEADER: &str = "fixture_id\tkind\tpolarity\ttool_binding\tinventory\tresult\texpected_diagnostic\texpected_count";
const SEED_PREFIX: u64 = 0x5350_5631_0000_0000;
const SEED_MASK: u64 = 0xffff_ffff_0000_0000;
const PHASES: &str = "Explicit,Reuse,Generate,Target,Shrink";
const HEGEL_CHECKSUM: &str = "f12603ff7e94c463c26e97565d725010e92139f68fc93edcacf7cb800cfd422b";
const FORBIDDEN_REFERENCE_MARKERS: [&str; 12] = [
    "buoy-core",
    "buoy_core",
    "buoy =",
    "buoy::",
    "../buoy-core",
    "spec-provenance-core",
    "spec_provenance_core",
    "spec-provenance =",
    "spec_provenance::",
    "crate::value",
    "crate::algorithms",
    "include!(",
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
struct PropertyRow {
    columns: Vec<String>,
}

impl PropertyRow {
    fn get(&self, index: usize) -> &str {
        &self.columns[index]
    }

    fn id(&self) -> &str {
        self.get(0)
    }
}

#[derive(Clone, Debug)]
struct RunRow {
    columns: Vec<String>,
}

impl RunRow {
    fn get(&self, index: usize) -> &str {
        &self.columns[index]
    }

    fn id(&self) -> &str {
        self.get(1)
    }
}

#[derive(Clone, Debug, Deserialize)]
struct ToolBinding {
    schema: String,
    repository: String,
    package: String,
    crate_name: String,
    version: String,
    source_commit: String,
    source_tree: String,
    archive_sha256: String,
    macro_package: String,
    macro_version: String,
    macro_sha256: String,
    c_package: String,
    c_version: String,
    c_source_commit: String,
    c_source_tree: String,
    c_sha256: String,
    license: String,
    toolchain_digest: String,
    dependency_boundary: String,
    offline: bool,
}

fn main() {
    std::process::exit(exit_code(run()));
}

fn exit_code(result: CheckResult<()>) -> i32 {
    match result {
        Ok(()) => 0,
        Err(error) => {
            eprintln!("property-evidence: {error}");
            1
        }
    }
}

fn run() -> CheckResult<()> {
    content_digest::self_test().expect("content-digest self-test must pass");
    let arguments = env::args().skip(1).collect::<Vec<_>>();
    dispatch(&arguments)
}

fn dispatch(arguments: &[String]) -> CheckResult<()> {
    match arguments {
        [] => validate_authoritative(),
        [argument] if argument == "--self-test" => self_test(),
        _ => Err(String::from("usage: check-property-evidence [--self-test]")),
    }
}

fn validate_authoritative() -> CheckResult<()> {
    validate_complete(Path::new("."), TOOL_BINDING, INVENTORY, RUNS, None, true)
        .and_then(emit_diagnostics)
        .and_then(|()| read(INVENTORY))
        .map(|inventory| {
            let count = inventory.lines().skip(1).count();
            println!("property-evidence: ok properties={count}");
        })
}

fn self_test() -> CheckResult<()> {
    let manifest = read(&fixture_path("manifest.tsv")).expect("fixture manifest must read");
    let mut lines = manifest.lines();
    require_conditions(
        &[lines.next() == Some(FIXTURE_MANIFEST_HEADER)],
        "HEGEL-NEGATIVE-CONTROL: fixture manifest header mismatch",
    )
    .expect("fixture manifest header must be valid");
    let positive_runs = parse_runs(
        &read(&fixture_path("results/positive.tsv")).expect("positive fixture results must read"),
    )
    .0;
    let positive_by_id = positive_runs
        .into_iter()
        .map(|row| (row.id().to_owned(), row))
        .collect::<BTreeMap<_, _>>();
    let source_root = Path::new(FIXTURE_ROOT).join("repository");
    let mut checked = 0_usize;
    for (index, line) in lines.enumerate() {
        let columns = line.split('\t').collect::<Vec<_>>();
        require_conditions(
            &[columns.len() == 8],
            &format!(
                "HEGEL-NEGATIVE-CONTROL: manifest line {} malformed",
                index + 2
            ),
        )
        .expect("fixture manifest row must be valid");
        let fixture_id = columns[0];
        let polarity = columns[2];
        let expected_code = columns[6];
        let expected_count = parse_expected_count(fixture_id, columns[7])
            .expect("fixture expected count must be an integer");
        let diagnostics = validate_complete(
            &source_root,
            &fixture_path(columns[3]),
            &fixture_path(columns[4]),
            &fixture_path(columns[5]),
            Some(&positive_by_id),
            false,
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
    property_controls();
    println!("property-evidence: self-test ok fixtures={checked}");
    Ok(())
}

fn require_conditions(conditions: &[bool], message: &str) -> CheckResult<()> {
    if conditions.iter().all(|condition| *condition) {
        Ok(())
    } else {
        Err(message.to_owned())
    }
}

fn parse_expected_count(fixture_id: &str, value: &str) -> CheckResult<usize> {
    value
        .parse::<usize>()
        .map_err(|error| format!("{fixture_id}: invalid expected count: {error}"))
}

fn expected_failure<T: std::fmt::Debug>(result: CheckResult<T>) {
    drop(result.expect_err("property-evidence negative control must fail"));
}

fn mutate_tsv_cell(source: &str, id_column: usize, id: &str, column: usize, value: &str) -> String {
    source
        .lines()
        .map(|line| {
            let mut columns = line.split('\t').collect::<Vec<_>>();
            if columns.get(id_column) == Some(&id) {
                columns[column] = value;
            }
            columns.join("\t")
        })
        .collect::<Vec<_>>()
        .join("\n")
        + "\n"
}

#[allow(clippy::too_many_lines)]
fn property_controls() {
    let _ = exit_code(Err(String::from("control")));
    require_conditions(&[true, true], "control").expect("true controls must pass");
    expected_failure(require_conditions(&[true, false], "control"));
    expected_failure(parse_expected_count("control", "wrong"));
    expected_failure(dispatch(&[String::from("invalid")]));

    let root = Path::new(FIXTURE_ROOT).join("repository");
    let tool_path = fixture_path("tools/positive.toml");
    let inventory_path = fixture_path("inventories/positive.tsv");
    let runs_path = fixture_path("results/positive.tsv");
    let tool = parse_tool(&tool_path).expect("positive tool must parse");
    let inventory_source = read(&inventory_path).expect("positive inventory must read");
    let runs_source = read(&runs_path).expect("positive runs must read");
    let (properties, _) =
        parse_inventory(&inventory_source, &root).expect("positive inventory must parse");
    let (runs, _) = parse_runs(&runs_source);
    let baseline = runs
        .iter()
        .cloned()
        .map(|row| (row.id().to_owned(), row))
        .collect::<BTreeMap<_, _>>();

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

    expected_failure(validate_complete(
        &root,
        "missing-tool.toml",
        &inventory_path,
        &runs_path,
        Some(&baseline),
        false,
    ));
    let control_root = Path::new("target/property-evidence-controls");
    fs::create_dir_all(control_root).expect("control directory must be writable");
    let malformed_tool_path = control_root.join("malformed.toml");
    fs::write(&malformed_tool_path, "not = [valid")
        .expect("malformed tool control must be writable");
    expected_failure(parse_tool(&malformed_tool_path.display().to_string()));
    expected_failure(validate_complete(
        &root,
        &malformed_tool_path.display().to_string(),
        &inventory_path,
        &runs_path,
        Some(&baseline),
        false,
    ));
    expected_failure(validate_complete(
        &root,
        &tool_path,
        "missing-inventory.tsv",
        &runs_path,
        Some(&baseline),
        false,
    ));
    expected_failure(validate_complete(
        &root,
        &tool_path,
        &inventory_path,
        "missing-runs.tsv",
        Some(&baseline),
        false,
    ));

    let mut changed_tool = tool.clone();
    changed_tool.schema = String::from("wrong");
    drop(validate_tool(&changed_tool, false));
    changed_tool = tool.clone();
    changed_tool.repository = String::from("wrong");
    drop(validate_tool(&changed_tool, false));
    changed_tool = tool.clone();
    changed_tool.offline = false;
    drop(validate_tool(&changed_tool, false));
    let _ =
        dependency_section_contains_hegel("[dependencies]\nhegel = \"x\"\n[dev-dependencies]\n");
    let _ = dependency_section_contains_hegel("[dependencies]\nserde = \"x\"\n");
    let _ = dependency_section_contains_hegel("no dependency section");
    let _ = toolchain_closure_valid(&tool, "wrong", "", "", "");
    drop(validate_toolchain_closure(&tool, "wrong", "", "", ""));
    drop(validate_tool_environment(
        &tool,
        [
            "missing-toolchain",
            "missing-lock",
            "missing-root",
            "missing-core",
        ],
    ));

    for source in [
        String::from("wrong\n"),
        format!("{INVENTORY_HEADER}\ntoo\tfew\n"),
    ] {
        drop(parse_inventory(&source, &root));
    }
    let first_property = properties[0].id().to_owned();
    for (column, value) in [
        (4, "a\\b.rs"),
        (7, "/absolute.rs"),
        (9, "a/../b.rs"),
        (16, ""),
        (11, "1"),
        (24, "proved"),
        (25, "nonproduction-control"),
    ] {
        drop(parse_inventory(
            &mutate_tsv_cell(&inventory_source, 0, &first_property, column, value),
            &root,
        ));
    }
    let changed_id_source = mutate_tsv_cell(&inventory_source, 0, &first_property, 0, "z-control");
    let reversed_inventory = format!(
        "{INVENTORY_HEADER}\n{}\n{}\n",
        changed_id_source.lines().nth(1).unwrap_or_default(),
        inventory_source.lines().nth(1).unwrap_or_default()
    );
    drop(parse_inventory(&reversed_inventory, &root));

    let nonproduction_target = mutate_tsv_cell(
        &mutate_tsv_cell(
            &inventory_source,
            0,
            &first_property,
            25,
            "nonproduction-control",
        ),
        0,
        &first_property,
        2,
        "named-target",
    );
    drop(parse_inventory(&nonproduction_target, &root));

    let production_property = properties[0].clone();
    for (column, value) in [
        (8, "missing_property"),
        (3, "production::missing"),
        (3, "production::other"),
        (6, "reference::other"),
    ] {
        let mut changed = production_property.clone();
        changed.columns[column] = value.to_owned();
        drop(validate_production_mapping(&root, &changed));
    }
    let mut missing_production_source = production_property.clone();
    missing_production_source.columns[4] = String::from("missing.rs");
    expected_failure(validate_production_mapping(
        &root,
        &missing_production_source,
    ));
    let mut missing_reference_source = production_property.clone();
    missing_reference_source.columns[7] = String::from("missing.rs");
    expected_failure(validate_reference_independence(
        &root,
        &missing_reference_source,
    ));

    for source in [
        String::from("duration\n"),
        String::from("host\n"),
        String::from("wrong\n"),
        format!("{RUN_HEADER}\ntoo\tfew\n"),
    ] {
        drop(parse_runs(&source));
    }
    let first_run = runs[0].id().to_owned();
    for (column, value) in [
        (15, "/absolute"),
        (15, "command /nix/store/tool"),
        (12, "/absolute-replay"),
        (13, "unknown"),
    ] {
        drop(parse_runs(&mutate_tsv_cell(
            &runs_source,
            1,
            &first_run,
            column,
            value,
        )));
    }
    let duplicate_run = format!(
        "{runs_source}{}\n",
        runs_source.lines().nth(1).unwrap_or_default()
    );
    drop(parse_runs(&duplicate_run));

    let property = properties[0].clone();
    let run = baseline[property.id()].clone();
    let one_property = [property.clone()];
    let one_run = [run.clone()];
    drop(validate_cross_records(
        &root,
        &tool,
        &one_property,
        &[],
        Some(&baseline),
        false,
    ));
    for (column, value) in [
        (0, "wrong-schema"),
        (2, "wrong-polarity"),
        (17, "proved"),
        (18, "missing"),
        (3, "wrong-tool"),
        (4, "wrong-version"),
        (5, "wrong-digest"),
        (16, "wrong-product"),
        (13, "fail"),
        (14, "wrong-diagnostic"),
        (19, "historical"),
    ] {
        let mut changed = run.clone();
        changed.columns[column] = value.to_owned();
        drop(validate_cross_records(
            &root,
            &tool,
            &one_property,
            &[changed],
            Some(&baseline),
            false,
        ));
    }
    let mut false_property = property.clone();
    false_property.columns[1] = String::from("fail");
    drop(validate_cross_records(
        &root,
        &tool,
        &[false_property.clone()],
        &one_run,
        Some(&baseline),
        false,
    ));
    let mut missing_failure_observation = run.clone();
    missing_failure_observation.columns[2] = String::from("fail");
    drop(validate_cross_records(
        &root,
        &tool,
        &[false_property.clone()],
        &[missing_failure_observation],
        Some(&baseline),
        false,
    ));
    let mut observed_failure = run.clone();
    observed_failure.columns[2] = String::from("fail");
    observed_failure.columns[13] = String::from("expected-failure-observed");
    observed_failure.columns[14] = String::from("HEGEL-NEGATIVE-CONTROL");
    drop(validate_cross_records(
        &root,
        &tool,
        &[false_property],
        &[observed_failure],
        Some(&baseline),
        false,
    ));
    drop(validate_cross_records(
        &root,
        &tool,
        &one_property,
        &one_run,
        Some(&BTreeMap::new()),
        false,
    ));
    drop(validate_cross_records(
        &root,
        &tool,
        &one_property,
        &one_run,
        None,
        false,
    ));
    for column in [9_usize, 10, 11] {
        let mut changed = run.clone();
        changed.columns[column] = String::from("none");
        drop(validate_fixture_freshness(&property, &changed, &baseline));
    }
    for column in [7_usize, 9, 10, 11] {
        let mut changed = run.clone();
        changed.columns[column] = String::from("sha-256:changed");
        drop(validate_fixture_freshness(&property, &changed, &baseline));
    }
    let mut nonproduction = property.clone();
    nonproduction.columns[25] = String::from("nonproduction-control");
    let mut no_production_binding = run.clone();
    no_production_binding.columns[10] = String::from("none");
    drop(validate_fixture_freshness(
        &nonproduction,
        &no_production_binding,
        &baseline,
    ));
    let mut no_reference = property.clone();
    no_reference.columns[5] = String::from("none");
    let mut no_reference_binding = run.clone();
    no_reference_binding.columns[11] = String::from("none");
    drop(validate_fixture_freshness(
        &no_reference,
        &no_reference_binding,
        &baseline,
    ));
    for (column, value) in [
        (9, "missing-property.rs"),
        (4, "missing-production.rs"),
        (7, "missing-reference.rs"),
        (22, "missing-replay.tsv"),
    ] {
        let mut missing_source = property.clone();
        if column == 7 {
            missing_source.columns[4] = String::from("none");
        }
        missing_source.columns[column] = value.to_owned();
        expected_failure(validate_authoritative_freshness(
            &root,
            &tool,
            &missing_source,
            &run,
        ));
    }
    expected_failure(validate_authoritative_freshness_with_toolchain(
        &root,
        &tool,
        &property,
        &run,
        Err(String::from("injected toolchain failure")),
    ));
    let mut stale_authoritative = run.clone();
    stale_authoritative.columns[7] = String::from("sha-256:changed");
    drop(validate_authoritative_freshness(
        &root,
        &tool,
        &property,
        &stale_authoritative,
    ));
    let missing_replay_inventory = mutate_tsv_cell(
        &inventory_source,
        0,
        &first_property,
        22,
        "missing-replay.tsv",
    );
    let missing_replay_inventory_path = control_root.join("missing-replay.tsv");
    let control_runs_path = control_root.join("runs.tsv");
    fs::write(&missing_replay_inventory_path, missing_replay_inventory)
        .expect("missing replay inventory control must be writable");
    fs::write(&control_runs_path, &runs_source).expect("run control must be writable");
    expected_failure(validate_complete(
        &root,
        TOOL_BINDING,
        &missing_replay_inventory_path.display().to_string(),
        &control_runs_path.display().to_string(),
        None,
        true,
    ));
    let missing_production_inventory = mutate_tsv_cell(
        &inventory_source,
        0,
        &first_property,
        4,
        "missing-production.rs",
    );
    let missing_production_inventory_path = control_root.join("missing-production.tsv");
    fs::write(
        &missing_production_inventory_path,
        missing_production_inventory,
    )
    .expect("missing production inventory control must be writable");
    expected_failure(validate_complete(
        &root,
        &tool_path,
        &missing_production_inventory_path.display().to_string(),
        &runs_path,
        Some(&baseline),
        false,
    ));

    let _ = api_declared("fn left() {}\nfn right() {}", "left/right");
    let _ = api_declared("fn left() {}", "left/right");
    let _ = api_invoked("left(); value.right(); Type::third", "left/right/third");
    let _ = api_invoked("left();", "left/right");
    let _ = api_leaf_names("module::left/right");
    let _ = function_body("fn control() { if true { value(); } }", "control");
    let _ = function_body("fn other() {}", "control");
    let _ = function_body("fn control()", "control");
    let _ = function_body("fn control() {", "control");
    optional_digest(&root, "none").expect("none digest must be supported");
    expected_failure(optional_digest(&root, "missing.rs"));
    for path in ["", "/absolute", "a\\b", "a/../b", "valid/path.rs"] {
        let _ = normalized_relative_path(path);
    }
    expected_failure(emit_diagnostics(vec![diagnostic]));
    emit_diagnostics(Vec::new()).expect("empty diagnostics must pass");
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
            "HEGEL-NEGATIVE-CONTROL: {fixture_id}: positive fixture failed: {}",
            render_diagnostics(diagnostics)
        ));
    }
    let observed = diagnostics
        .iter()
        .filter(|diagnostic| diagnostic.code == expected_code)
        .count();
    if observed != expected_count || diagnostics.len() != expected_count {
        return Err(format!(
            "HEGEL-NEGATIVE-CONTROL: {fixture_id}: expected {expected_count} `{expected_code}`, observed {observed}/{}: {}",
            diagnostics.len(),
            render_diagnostics(diagnostics)
        ));
    }
    Ok(())
}

fn validate_complete(
    source_root: &Path,
    tool_path: &str,
    inventory_path: &str,
    runs_path: &str,
    fixture_baseline: Option<&BTreeMap<String, RunRow>>,
    authoritative: bool,
) -> CheckResult<Vec<Diagnostic>> {
    let tool = parse_tool(tool_path)?;
    let mut diagnostics = validate_tool(&tool, authoritative);
    if !diagnostics.is_empty() {
        diagnostics.sort();
        return Ok(diagnostics);
    }
    let (properties, mut inventory_diagnostics) =
        parse_inventory(&read(inventory_path)?, source_root)?;
    diagnostics.append(&mut inventory_diagnostics);
    if !diagnostics.is_empty() {
        diagnostics.sort();
        return Ok(diagnostics);
    }
    let (runs, mut run_diagnostics) = parse_runs(&read(runs_path)?);
    diagnostics.append(&mut run_diagnostics);
    if !diagnostics.is_empty() {
        diagnostics.sort();
        return Ok(diagnostics);
    }
    diagnostics.extend(validate_cross_records(
        source_root,
        &tool,
        &properties,
        &runs,
        fixture_baseline,
        authoritative,
    )?);
    diagnostics.sort();
    Ok(diagnostics)
}

fn validate_tool(tool: &ToolBinding, authoritative: bool) -> Vec<Diagnostic> {
    let exact = [
        tool.schema == "buoy-hegel-tool-binding/v1",
        tool.repository == "https://github.com/hegeldev/hegel-rust",
        tool.package == "hegeltest",
        tool.crate_name == "hegel",
        tool.version == "0.28.0",
        tool.source_commit == "634ea934be19dbb86a5792b4c3d35d6d4a54f7ad",
        tool.source_tree == "dc6a62465fb3f6b8d34942c60637d2d1aaca4347",
        tool.archive_sha256 == HEGEL_CHECKSUM,
        tool.macro_package == "hegeltest-macros",
        tool.macro_version == "0.28.0",
        tool.macro_sha256 == "c3fe342530584e9e0ac8144d40a746fff47c63ad1a2725c8793fbc883f7a6f9e",
        tool.c_package == "hegeltest-c",
        tool.c_version == "0.29.0",
        tool.c_source_commit == "b8e2fdcf7046b55fdc84d9b7be68250324654f74",
        tool.c_source_tree == "3736ecbe3cd96918c3d1aa19c815eb8ba6bcc4d2",
        tool.c_sha256 == "dc2ab96fd030ce7ad37081c1f6ab3186ba794b9894cefeef2644d9dc1ecc1df7",
        tool.license == "MIT",
        tool.dependency_boundary == "test-shell-only",
        tool.offline,
    ]
    .into_iter()
    .all(|matches| matches);
    if !exact {
        return vec![Diagnostic::new(
            "HEGEL-TOOL-IDENTITY",
            "hegeltest",
            "tool binding differs from the exact approved package closure",
        )];
    }
    if authoritative {
        return validate_tool_environment(
            tool,
            [
                "rust-toolchain.toml",
                "Cargo.lock",
                "Cargo.toml",
                "crates/buoy-core/Cargo.toml",
            ],
        );
    }
    Vec::new()
}

fn validate_tool_environment(tool: &ToolBinding, paths: [&str; 4]) -> Vec<Diagnostic> {
    let closure = content_digest::file_binding(Path::new(paths[0])).and_then(|toolchain| {
        read(paths[1]).and_then(|lock| {
            read(paths[2]).and_then(|root_manifest| {
                read(paths[3]).map(|core_manifest| (toolchain, lock, root_manifest, core_manifest))
            })
        })
    });
    match closure {
        Ok((toolchain, lock, root_manifest, core_manifest)) => {
            validate_toolchain_closure(tool, &toolchain, &lock, &root_manifest, &core_manifest)
        }
        Err(error) => vec![Diagnostic::new(
            "HEGEL-TOOL-IDENTITY",
            "hegeltest",
            format!("cannot validate Cargo/toolchain closure: {error}"),
        )],
    }
}

fn validate_toolchain_closure(
    tool: &ToolBinding,
    toolchain: &str,
    lock: &str,
    root_manifest: &str,
    core_manifest: &str,
) -> Vec<Diagnostic> {
    if toolchain_closure_valid(tool, toolchain, lock, root_manifest, core_manifest) {
        Vec::new()
    } else {
        vec![Diagnostic::new(
            "HEGEL-TOOL-IDENTITY",
            "hegeltest",
            "Cargo/toolchain closure or test-only dependency boundary is stale",
        )]
    }
}

fn toolchain_closure_valid(
    tool: &ToolBinding,
    toolchain: &str,
    lock: &str,
    root_manifest: &str,
    core_manifest: &str,
) -> bool {
    ![
        tool.toolchain_digest != toolchain,
        !lock.contains(&format!(
            "name = \"hegeltest\"\nversion = \"0.28.0\"\nsource = \"registry+https://github.com/rust-lang/crates.io-index\"\nchecksum = \"{HEGEL_CHECKSUM}\""
        )),
        !root_manifest.contains("hegeltest = \"=0.28.0\""),
        !core_manifest.contains("[dev-dependencies]\nhegeltest = \"=0.28.0\""),
        dependency_section_contains_hegel(core_manifest),
    ]
    .contains(&true)
}

fn dependency_section_contains_hegel(manifest: &str) -> bool {
    manifest
        .split("[dependencies]")
        .nth(1)
        .and_then(|tail| tail.split("[dev-dependencies]").next())
        .is_some_and(|dependencies| dependencies.contains("hegel"))
}

fn parse_inventory(
    source: &str,
    source_root: &Path,
) -> CheckResult<(Vec<PropertyRow>, Vec<Diagnostic>)> {
    let mut diagnostics = Vec::new();
    let mut lines = source.lines();
    if lines.next() != Some(INVENTORY_HEADER) {
        return Ok((
            Vec::new(),
            vec![Diagnostic::new(
                "HEGEL-INVENTORY-HEADER",
                "inventory",
                "header does not match the closed property schema",
            )],
        ));
    }
    let mut rows = Vec::new();
    let mut ids = BTreeSet::new();
    let mut seeds = BTreeSet::new();
    let mut mappings = BTreeSet::new();
    let mut previous_id: Option<String> = None;
    for (index, line) in lines.enumerate() {
        let columns = line.split('\t').map(str::to_owned).collect::<Vec<_>>();
        if columns.len() != 26 {
            diagnostics.push(Diagnostic::new(
                "HEGEL-INVENTORY-ROW",
                format!("line-{}", index + 2),
                "property row must contain 26 columns",
            ));
            continue;
        }
        let row = PropertyRow { columns };
        if !ids.insert(row.id().to_owned()) {
            diagnostics.push(Diagnostic::new(
                "HEGEL-PROPERTY-DUPLICATE-ID",
                row.id(),
                "property ID appears more than once",
            ));
        }
        if previous_id
            .as_ref()
            .is_some_and(|previous| previous.as_str() > row.id())
        {
            diagnostics.push(Diagnostic::new(
                "HEGEL-INVENTORY-ORDER",
                row.id(),
                "property rows are not strictly ordered",
            ));
        }
        previous_id = Some(row.id().to_owned());
        let seed = row.get(10).parse::<u64>().unwrap_or(0);
        if !seeds.insert(seed) {
            diagnostics.push(Diagnostic::new(
                "HEGEL-SEED-DUPLICATE",
                row.id(),
                format!("seed `{seed}` appears more than once"),
            ));
        }
        for path_index in [4_usize, 7, 9, 22] {
            let path = row.get(path_index);
            if path != "none" && !normalized_relative_path(path) {
                diagnostics.push(Diagnostic::new(
                    "HEGEL-INVENTORY-PATH",
                    row.id(),
                    format!(
                        "column {path_index} path `{path}` is not normalized repository-relative"
                    ),
                ));
            }
        }
        if [row.get(16), row.get(17), row.get(18), row.get(19)].contains(&"") {
            diagnostics.push(Diagnostic::new(
                "HEGEL-DOMAIN-MISSING",
                row.id(),
                "generator domain, limits, edges, and rejection limit are mandatory",
            ));
        }
        if [
            seed & SEED_MASK != SEED_PREFIX,
            row.get(11) != "256",
            row.get(12) != "Default",
            row.get(13) != "true",
            row.get(14) != "none",
            row.get(15) != PHASES,
        ]
        .contains(&true)
        {
            diagnostics.push(Diagnostic::new(
                "HEGEL-SETTINGS-AMBIENT",
                row.id(),
                "runner settings differ from the exact committed configuration",
            ));
        }
        if row.get(24) != "sampled" {
            diagnostics.push(Diagnostic::new(
                "HEGEL-ASSURANCE-INFLATION",
                row.id(),
                "property evidence must remain sampled",
            ));
        }
        if row.get(25).starts_with("production-") {
            let mapping = (row.get(2).to_owned(), row.get(8).to_owned());
            if !mappings.insert(mapping) {
                diagnostics.push(Diagnostic::new(
                    "HEGEL-PROPERTY-DUPLICATE-MAPPING",
                    row.id(),
                    "production target/test mapping appears more than once",
                ));
            }
            diagnostics.extend(validate_production_mapping(source_root, &row)?);
        } else if row.get(2) != "none" {
            diagnostics.push(Diagnostic::new(
                "HEGEL-PRODUCTION-NOT-CALLED",
                row.id(),
                "nonproduction property cannot close a named production target",
            ));
        }
        if row.get(5) == "independent-reference" {
            diagnostics.extend(validate_reference_independence(source_root, &row)?);
        }
        rows.push(row);
    }
    diagnostics.sort();
    Ok((rows, diagnostics))
}

fn validate_production_mapping(
    source_root: &Path,
    row: &PropertyRow,
) -> CheckResult<Vec<Diagnostic>> {
    let production_path = source_root.join(row.get(4));
    let test_path = source_root.join(row.get(9));
    let production_source = read_path(&production_path)?;
    let test_source = read_path(&test_path)?;
    let Some(body) = function_body(&test_source, row.get(8)) else {
        return Ok(vec![Diagnostic::new(
            "HEGEL-PROPERTY-MISSING",
            row.id(),
            format!("test function `{}` is absent", row.get(8)),
        )]);
    };
    if !api_declared(&production_source, row.get(3)) {
        return Ok(vec![Diagnostic::new(
            "HEGEL-PROPERTY-MISSING",
            row.id(),
            format!("production API `{}` is absent", row.get(3)),
        )]);
    }
    if !api_invoked(body, row.get(3)) {
        return Ok(vec![Diagnostic::new(
            "HEGEL-PRODUCTION-NOT-CALLED",
            row.id(),
            format!("test does not invoke `{}`", row.get(3)),
        )]);
    }
    if row.get(5) == "independent-reference" && !api_invoked(body, row.get(6)) {
        return Ok(vec![Diagnostic::new(
            "HEGEL-REFERENCE-NOT-CALLED",
            row.id(),
            format!("test does not invoke `{}`", row.get(6)),
        )]);
    }
    Ok(Vec::new())
}

fn validate_reference_independence(
    source_root: &Path,
    row: &PropertyRow,
) -> CheckResult<Vec<Diagnostic>> {
    let source = read_path(&source_root.join(row.get(7)))?;
    for marker in FORBIDDEN_REFERENCE_MARKERS {
        if source.contains(marker) {
            return Ok(vec![Diagnostic::new(
                "HEGEL-REFERENCE-NOT-INDEPENDENT",
                row.id(),
                format!("reference source contains forbidden marker `{marker}`"),
            )]);
        }
    }
    Ok(Vec::new())
}

fn parse_runs(source: &str) -> (Vec<RunRow>, Vec<Diagnostic>) {
    let mut diagnostics = Vec::new();
    let mut lines = source.lines();
    let header = lines.next().unwrap_or_default();
    if [
        header.contains("timestamp"),
        header.contains("duration"),
        header.contains("host"),
    ]
    .contains(&true)
    {
        return (
            Vec::new(),
            vec![Diagnostic::new(
                "HEGEL-RESULT-HOST-FIELD",
                "results",
                "normalized result declares a host-dependent field",
            )],
        );
    }
    if header != RUN_HEADER {
        return (
            Vec::new(),
            vec![Diagnostic::new(
                "HEGEL-RESULT-SCHEMA",
                "results",
                "header does not match buoy-property-result/v1",
            )],
        );
    }
    let mut rows = Vec::new();
    let mut ids = BTreeSet::new();
    let mut previous_id: Option<String> = None;
    for (index, line) in lines.enumerate() {
        let columns = line.split('\t').map(str::to_owned).collect::<Vec<_>>();
        if columns.len() != 20 {
            diagnostics.push(Diagnostic::new(
                "HEGEL-RESULT-SCHEMA",
                format!("line-{}", index + 2),
                "result row must contain 20 columns",
            ));
            continue;
        }
        let row = RunRow { columns };
        if !ids.insert(row.id().to_owned()) {
            diagnostics.push(Diagnostic::new(
                "HEGEL-RESULT-DUPLICATE",
                row.id(),
                "result property ID appears more than once",
            ));
        }
        if previous_id
            .as_ref()
            .is_some_and(|previous| previous.as_str() >= row.id())
        {
            diagnostics.push(Diagnostic::new(
                "HEGEL-RESULT-ORDER",
                row.id(),
                "result rows are not strictly ordered",
            ));
        }
        previous_id = Some(row.id().to_owned());
        if [
            row.get(15).starts_with('/'),
            row.get(15).contains("/tmp/"),
            row.get(15).contains("/nix/store/"),
        ]
        .contains(&true)
        {
            diagnostics.push(Diagnostic::new(
                "HEGEL-RESULT-HOST-FIELD",
                row.id(),
                "replay command contains a host-dependent path",
            ));
        }
        if [row.get(12).starts_with('/'), row.get(12).contains("shrink")].contains(&true) {
            diagnostics.push(Diagnostic::new(
                "HEGEL-REPLAY-UNCOMMITTED",
                row.id(),
                "raw or uncommitted shrink output is not canonical replay evidence",
            ));
        }
        if !matches!(row.get(13), "pass" | "expected-failure-observed" | "fail") {
            diagnostics.push(Diagnostic::new(
                "HEGEL-EXECUTION-INCOMPLETE",
                row.id(),
                format!("unexpected execution state `{}`", row.get(13)),
            ));
        }
        rows.push(row);
    }
    diagnostics.sort();
    (rows, diagnostics)
}

fn validate_cross_records(
    source_root: &Path,
    tool: &ToolBinding,
    properties: &[PropertyRow],
    runs: &[RunRow],
    fixture_baseline: Option<&BTreeMap<String, RunRow>>,
    authoritative: bool,
) -> CheckResult<Vec<Diagnostic>> {
    let mut diagnostics = Vec::new();
    let property_by_id = properties
        .iter()
        .map(|row| (row.id(), row))
        .collect::<BTreeMap<_, _>>();
    let run_by_id = runs
        .iter()
        .map(|row| (row.id(), row))
        .collect::<BTreeMap<_, _>>();
    if property_by_id.keys().collect::<Vec<_>>() != run_by_id.keys().collect::<Vec<_>>() {
        diagnostics.push(Diagnostic::new(
            "HEGEL-PROPERTY-MISSING",
            "inventory-results",
            "property and normalized result ID sets differ",
        ));
        return Ok(diagnostics);
    }
    for (id, property) in property_by_id {
        let run = run_by_id[id];
        if [
            run.get(0) != "buoy-property-result/v1",
            run.get(2) != property.get(1),
            run.get(17) != "sampled",
            run.get(18) != "present",
        ]
        .contains(&true)
        {
            diagnostics.push(Diagnostic::new(
                "HEGEL-RESULT-BINDING-MISSING",
                id,
                "result does not match inventory/settings/sample registration",
            ));
            continue;
        }
        if [
            run.get(3) != "hegeltest",
            run.get(4) != "0.28.0",
            run.get(5) != format!("sha-256:{HEGEL_CHECKSUM}"),
            run.get(16) != "buoy-0.2.0-post-rename",
        ]
        .contains(&true)
        {
            diagnostics.push(Diagnostic::new(
                "HEGEL-TOOL-IDENTITY",
                id,
                "result tool or product identity is stale",
            ));
            continue;
        }
        if property.get(1) == "fail" {
            if [
                run.get(13) != "expected-failure-observed",
                run.get(14) != "HEGEL-NEGATIVE-CONTROL",
            ]
            .contains(&true)
            {
                diagnostics.push(Diagnostic::new(
                    "HEGEL-NEGATIVE-CONTROL",
                    id,
                    "expected false property rejection is absent",
                ));
            }
        } else if [run.get(13) != "pass", run.get(14) != "none"].contains(&true) {
            diagnostics.push(Diagnostic::new(
                "HEGEL-EXECUTION-INCOMPLETE",
                id,
                "passing property lacks a complete passing verdict",
            ));
        }
        if run.get(19) != "current" {
            diagnostics.push(Diagnostic::new(
                "HEGEL-EVIDENCE-STALE",
                id,
                "result freshness is not current",
            ));
        }
        if authoritative {
            diagnostics.extend(validate_authoritative_freshness(
                source_root,
                tool,
                property,
                run,
            )?);
        } else if let Some(baseline) = fixture_baseline {
            diagnostics.extend(validate_fixture_freshness(property, run, baseline));
        }
    }
    Ok(diagnostics)
}

fn validate_authoritative_freshness(
    source_root: &Path,
    tool: &ToolBinding,
    property: &PropertyRow,
    run: &RunRow,
) -> CheckResult<Vec<Diagnostic>> {
    validate_authoritative_freshness_with_toolchain(
        source_root,
        tool,
        property,
        run,
        content_digest::file_binding(Path::new("rust-toolchain.toml")),
    )
}

fn validate_authoritative_freshness_with_toolchain(
    source_root: &Path,
    tool: &ToolBinding,
    property: &PropertyRow,
    run: &RunRow,
    toolchain: CheckResult<String>,
) -> CheckResult<Vec<Diagnostic>> {
    let seed = property.get(10).parse::<u64>().unwrap_or(0);
    let settings = format!(
        "backend={}\ncases={}\nseed={seed}\nderandomize={}\ndatabase={}\nphases={}\ndomain={}\nlimits={}\nedges={}\nrejection_limit={}\n",
        property.get(12),
        property.get(11),
        property.get(13),
        property.get(14),
        property.get(15),
        property.get(16),
        property.get(17),
        property.get(18),
        property.get(19)
    );
    let domain = format!(
        "domain={}\nlimits={}\nedges={}\nrejection_limit={}\nscope={}\n",
        property.get(16),
        property.get(17),
        property.get(18),
        property.get(19),
        property.get(23)
    );
    let expected = [
        content_digest::bytes_binding(settings.as_bytes()),
        content_digest::bytes_binding(domain.as_bytes()),
        content_digest::file_binding(&source_root.join(property.get(9)))?,
        optional_digest(source_root, property.get(4))?,
        optional_digest(source_root, property.get(7))?,
        optional_digest(source_root, property.get(22))?,
    ];
    let actual = [
        run.get(7),
        run.get(8),
        run.get(9),
        run.get(10),
        run.get(11),
        run.get(12),
    ];
    let toolchain = toolchain?;
    let bindings_changed = expected
        .iter()
        .map(String::as_str)
        .zip(actual)
        .any(|(left, right)| left != right);
    if [
        bindings_changed,
        run.get(6) != toolchain,
        run.get(6) != tool.toolchain_digest,
    ]
    .contains(&true)
    {
        return Ok(vec![Diagnostic::new(
            "HEGEL-EVIDENCE-STALE",
            property.id(),
            "source/settings/domain/reference/replay/toolchain binding changed",
        )]);
    }
    Ok(Vec::new())
}

fn validate_fixture_freshness(
    property: &PropertyRow,
    run: &RunRow,
    baseline: &BTreeMap<String, RunRow>,
) -> Vec<Diagnostic> {
    let Some(expected) = baseline.get(run.id()) else {
        return vec![Diagnostic::new(
            "HEGEL-RESULT-BINDING-MISSING",
            run.id(),
            "fixture result has no positive baseline",
        )];
    };
    let production_required = property.get(25).starts_with("production-");
    let reference_required = property.get(5) == "independent-reference";
    if [
        run.get(9) == "none",
        production_required && run.get(10) == "none",
        reference_required && run.get(11) == "none",
    ]
    .contains(&true)
    {
        return vec![Diagnostic::new(
            "HEGEL-RESULT-BINDING-MISSING",
            property.id(),
            "required property/production/reference binding is absent",
        )];
    }
    for index in [7_usize, 9, 10, 11] {
        if run.get(index) != expected.get(index) {
            return vec![Diagnostic::new(
                "HEGEL-EVIDENCE-STALE",
                property.id(),
                format!("fixture binding column {index} differs from the positive baseline"),
            )];
        }
    }
    Vec::new()
}

fn api_declared(source: &str, api: &str) -> bool {
    api_leaf_names(api)
        .iter()
        .all(|name| source.contains(&format!("fn {name}")))
}

fn api_invoked(body: &str, api: &str) -> bool {
    api_leaf_names(api)
        .iter()
        .all(|name| body.contains(&format!("{name}(")) || body.contains(&format!("::{name}")))
}

fn api_leaf_names(api: &str) -> Vec<&str> {
    api.rsplit("::").next().unwrap_or(api).split('/').collect()
}

fn function_body<'a>(source: &'a str, name: &str) -> Option<&'a str> {
    let marker = format!("fn {name}(");
    let start = source.find(&marker)?;
    let brace = source[start..].find('{')? + start;
    let mut depth = 0_usize;
    for (offset, byte) in source.as_bytes()[brace..].iter().enumerate() {
        match byte {
            b'{' => depth += 1,
            b'}' => {
                depth -= 1;
                if depth == 0 {
                    return source.get(brace..=brace + offset);
                }
            }
            _ => {}
        }
    }
    None
}

fn optional_digest(root: &Path, path: &str) -> CheckResult<String> {
    if path == "none" {
        Ok(String::from("not-applicable"))
    } else {
        content_digest::file_binding(&root.join(path))
    }
}

fn parse_tool(path: &str) -> CheckResult<ToolBinding> {
    toml::from_str(&read(path)?)
        .map_err(|error| format!("HEGEL-TOOL-IDENTITY: cannot parse {path}: {error}"))
}

fn normalized_relative_path(path: &str) -> bool {
    let value = Path::new(path);
    !path.is_empty()
        && !value.is_absolute()
        && !path.contains('\\')
        && value
            .components()
            .all(|component| matches!(component, Component::Normal(_)))
}

fn emit_diagnostics(mut diagnostics: Vec<Diagnostic>) -> CheckResult<()> {
    diagnostics.sort();
    if diagnostics.is_empty() {
        Ok(())
    } else {
        Err(render_diagnostics(&diagnostics))
    }
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

fn read(path: &str) -> CheckResult<String> {
    read_path(Path::new(path))
}

fn read_path(path: &Path) -> CheckResult<String> {
    fs::read_to_string(path).map_err(|error| format!("cannot read {}: {error}", path.display()))
}
