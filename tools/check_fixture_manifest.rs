//! Strict phase-3 fixture contract and independent-model checker.

#![allow(
    clippy::assigning_clones,
    clippy::bool_comparison,
    clippy::drop_non_drop,
    clippy::expect_used,
    clippy::match_same_arms,
    clippy::needless_pass_by_value,
    clippy::too_many_lines,
    clippy::unnecessary_wraps,
    clippy::while_let_on_iterator
)]

use std::collections::{BTreeMap, BTreeSet, VecDeque};
use std::env;
use std::fs;
use std::path::{Path, PathBuf};

const CASE_ROOTS: [&str; 4] = [
    "fixtures/v2/spg",
    "fixtures/v2/det",
    "fixtures/v2/evd",
    "fixtures/v2/fpr",
];
const MODEL_ROOTS: [&str; 3] = [
    "fixtures/v2/models",
    "fixtures/hardening/v2/models",
    "fixtures/chelis-provenance/v1/models",
];
const BASELINE_ROOT: &str = "fixtures/v2/baselines";
const README: &str = "fixtures/v2/README.md";
const MANIFEST: &str = "fixtures/v2/manifest.tsv";
const SPEC_ROOT: &str =
    "openspec/changes/archive/2026-07-19-establish-spec-provenance-system/specs";
const CASE_SCHEMA: &str = "spec-provenance-fixture-cases/v2";
const BASELINE_SCHEMA: &str = "spec-provenance-fixture-baseline/v2";
const MODEL_SCHEMA: &str = "spec-provenance-reference-model/v2";
const FINAL_ORACLE: &str = "devenv test";

type CheckResult<T> = Result<T, String>;

#[derive(Clone, Debug)]
struct Case {
    id: String,
    requirement: String,
    scenario: String,
    polarity: String,
    operation: String,
    baseline: String,
    mutations: Vec<String>,
    outcome: String,
    codes: Vec<String>,
    facts: Vec<String>,
    fixture: String,
}

#[derive(Clone, Debug)]
struct SpecScenario {
    slug: String,
    match_key: String,
    polarity: String,
}

#[derive(Default)]
struct CaseBuilder {
    id: Option<String>,
    scenario: Option<String>,
    polarity: Option<String>,
    operation: Option<String>,
    baseline: Option<String>,
    mutations: Option<Vec<String>>,
    outcome: Option<String>,
    codes: Option<Vec<String>>,
    facts: Option<Vec<String>>,
}

impl CaseBuilder {
    fn finish(self, requirement: &str, fixture: &str) -> CheckResult<Case> {
        let field = |value: Option<String>, name: &str| {
            value.ok_or_else(|| format!("{fixture}: case is missing `{name}`"))
        };
        Ok(Case {
            id: field(self.id, "id")?,
            requirement: requirement.to_owned(),
            scenario: field(self.scenario, "scenario")?,
            polarity: field(self.polarity, "polarity")?,
            operation: field(self.operation, "operation")?,
            baseline: field(self.baseline, "baseline")?,
            mutations: self
                .mutations
                .ok_or_else(|| format!("{fixture}: case is missing `mutations`"))?,
            outcome: field(self.outcome, "expected.outcome")?,
            codes: self
                .codes
                .ok_or_else(|| format!("{fixture}: case is missing `expected.codes`"))?,
            facts: self
                .facts
                .ok_or_else(|| format!("{fixture}: case is missing `expected.facts`"))?,
            fixture: fixture.to_owned(),
        })
    }
}

#[derive(Debug, Default)]
struct Predicate {
    name: String,
    clauses: Vec<Vec<String>>,
}

#[derive(Debug, Default)]
struct ModelInvariant {
    id: String,
    relation: Option<String>,
    if_all: Vec<String>,
    then_all: Vec<String>,
}

#[derive(Debug, Default)]
struct ModelState {
    name: String,
    terminal: bool,
    id_reserved: bool,
}

#[derive(Debug, Default)]
struct Transition {
    from: String,
    event: String,
    to: String,
}

#[derive(Debug)]
struct ModelResult {
    id: String,
    kind: String,
    state_count: usize,
    invariant_count: usize,
}

#[derive(Debug, Default)]
struct Model {
    id: String,
    kind: String,
    requirement_ids: Vec<String>,
    expected_state_count: Option<usize>,
    expected_reachable_count: Option<usize>,
    initial: Vec<String>,
    dimensions: Vec<(String, Vec<String>)>,
    predicates: Vec<Predicate>,
    states: Vec<ModelState>,
    transitions: Vec<Transition>,
    invariants: Vec<ModelInvariant>,
}

#[derive(Clone, Copy)]
enum ModelSection {
    Top,
    Dimension(usize),
    Predicate(usize),
    Clause(usize, usize),
    State(usize),
    Transition(usize),
    Invariant(usize),
}

fn main() {
    std::process::exit(exit_code(run()));
}

fn exit_code(result: CheckResult<()>) -> i32 {
    match result {
        Ok(()) => 0,
        Err(error) => {
            eprintln!("fixture contract check failed: {error}");
            1
        }
    }
}

fn run() -> CheckResult<()> {
    let arguments = env::args().skip(1).collect::<Vec<_>>();
    run_arguments(&arguments)
}

fn validate_repository(write_manifest: bool) -> CheckResult<()> {
    parse_vocabularies(Path::new(README)).and_then(|(operations, mutations)| {
        read_cases(&operations, &mutations).and_then(|cases| {
            validate_baselines(&cases)
                .and_then(|()| read_spec_scenarios())
                .and_then(|scenarios| render_manifest(&cases, &scenarios))
                .and_then(|manifest| {
                    validate_manifest_file(Path::new(MANIFEST), &manifest, write_manifest)
                })
                .and_then(|()| validate_models())
                .map(|model_results| report_repository(&cases, &model_results))
        })
    })
}

fn report_repository(cases: &[Case], model_results: &[ModelResult]) {
    let state_count: usize = model_results.iter().map(|result| result.state_count).sum();
    for result in model_results {
        println!(
            "model result: id={} kind={} states={} invariants={} fixed-point=true",
            result.id, result.kind, result.state_count, result.invariant_count
        );
    }
    let positive = cases
        .iter()
        .filter(|case| case.polarity == "positive")
        .count();
    let negative = cases.len() - positive;
    println!(
        "fixture contracts: {} cases ({} positive, {} negative), {} models, {} finite states",
        cases.len(),
        positive,
        negative,
        model_results.len(),
        state_count
    );
}

fn validate_manifest_file(path: &Path, manifest: &str, write_manifest: bool) -> CheckResult<()> {
    if write_manifest {
        fs::write(path, manifest).map_err(|error| format!("write {}: {error}", path.display()))
    } else {
        let current = fs::read_to_string(path).map_err(|error| {
            format!(
                "read {}: {error}; run with --write-manifest once",
                path.display()
            )
        })?;
        if current == manifest {
            Ok(())
        } else {
            Err(format!(
                "{} is stale; review fixture changes and regenerate with --write-manifest",
                path.display()
            ))
        }
    }
}

fn io_error(error: std::io::Error) -> String {
    error.to_string()
}

fn control_success<T>(result: CheckResult<T>) -> T {
    result.expect("fixture-manifest positive control must succeed")
}

fn expected_failure<T: std::fmt::Debug>(result: CheckResult<T>) {
    drop(result.expect_err("fixture-manifest negative control must fail"));
}

fn control_case_builder() -> CaseBuilder {
    CaseBuilder {
        id: Some(String::from("control-case")),
        scenario: Some(String::from("positive-control")),
        polarity: Some(String::from("positive")),
        operation: Some(String::from("control")),
        baseline: Some(String::from("control.toml")),
        mutations: Some(Vec::new()),
        outcome: Some(String::from("accept")),
        codes: Some(Vec::new()),
        facts: Some(vec![String::from("control=true")]),
    }
}

fn control_case() -> Case {
    Case {
        id: String::from("control-case"),
        requirement: String::from("SPG-CONTROL"),
        scenario: String::from("positive-control"),
        polarity: String::from("positive"),
        operation: String::from("control"),
        baseline: String::from("control.toml"),
        mutations: Vec::new(),
        outcome: String::from("accept"),
        codes: Vec::new(),
        facts: vec![String::from("control=true")],
        fixture: String::from("control.toml"),
    }
}

fn control_truth_model() -> Model {
    Model {
        id: String::from("truth-control"),
        kind: String::from("truth-table"),
        requirement_ids: vec![String::from("SPG-CONTROL")],
        expected_state_count: Some(2),
        dimensions: vec![(
            String::from("value"),
            vec![String::from("a"), String::from("b")],
        )],
        predicates: vec![Predicate {
            name: String::from("is-a"),
            clauses: vec![vec![String::from("value=a")]],
        }],
        invariants: vec![ModelInvariant {
            id: String::from("truth-invariant"),
            relation: None,
            if_all: vec![String::from("value=a")],
            then_all: vec![String::from("is-a=true")],
        }],
        ..Model::default()
    }
}

fn control_transition_model() -> Model {
    Model {
        id: String::from("transition-control"),
        kind: String::from("transition-system"),
        requirement_ids: vec![String::from("SPG-CONTROL")],
        expected_reachable_count: Some(2),
        initial: vec![String::from("live")],
        states: vec![
            ModelState {
                name: String::from("live"),
                terminal: false,
                id_reserved: true,
            },
            ModelState {
                name: String::from("dead"),
                terminal: true,
                id_reserved: true,
            },
        ],
        transitions: vec![Transition {
            from: String::from("live"),
            event: String::from("stop"),
            to: String::from("dead"),
        }],
        invariants: vec![
            ModelInvariant {
                id: String::from("reachable"),
                relation: Some(String::from("all-states-reachable")),
                ..ModelInvariant::default()
            },
            ModelInvariant {
                id: String::from("terminal"),
                relation: Some(String::from("terminal-has-no-outgoing")),
                ..ModelInvariant::default()
            },
            ModelInvariant {
                id: String::from("reserved"),
                relation: Some(String::from("id-remains-reserved")),
                ..ModelInvariant::default()
            },
        ],
        ..Model::default()
    }
}

#[allow(clippy::too_many_lines)]
fn self_test() -> CheckResult<()> {
    let _ = exit_code(Err(String::from("control")));
    expected_failure(run_arguments(&[String::from("invalid")]));
    drop(control_success(parse_arguments(&[String::from(
        "--write-manifest",
    )])));
    expected_failure(run_arguments(&[
        String::from("invalid"),
        String::from("extra"),
    ]));
    let root = Path::new("target/fixture-manifest-controls");
    drop(fs::remove_dir_all(root));
    control_success(fs::create_dir_all(root).map_err(io_error));

    let manifest_control = root.join("manifest.tsv");
    control_success(validate_manifest_file(&manifest_control, "control\n", true));
    control_success(validate_manifest_file(
        &manifest_control,
        "control\n",
        false,
    ));
    expected_failure(validate_manifest_file(
        &manifest_control,
        "changed\n",
        false,
    ));
    expected_failure(validate_manifest_file(root, "control\n", true));
    let missing_manifest = root.join("missing-manifest.tsv");
    expected_failure(validate_manifest_file(
        &missing_manifest,
        "control\n",
        false,
    ));
    drop(io_error(
        fs::read_to_string(&missing_manifest).expect_err("missing control must fail"),
    ));

    let vocabulary = root.join("README.md");
    control_success(fs::write(
        &vocabulary,
        "## SPG operations\n- `control`\n## SPG mutations\n- `change`\n## Expected facts\n- `ignored`\n",
    )
    .map_err(io_error));
    drop(control_success(parse_vocabularies(&vocabulary)));
    let empty_vocabulary = root.join("empty.md");
    control_success(fs::write(&empty_vocabulary, "## SPG operations\n").map_err(io_error));
    expected_failure(parse_vocabularies(&empty_vocabulary));
    expected_failure(parse_vocabularies(&root.join("missing.md")));
    for line in [
        "not a bullet",
        "- `valid`",
        "- `bad<value`",
        "- `bad=value`",
        "- `bad:value`",
        "- `unterminated",
    ] {
        drop(backtick_bullet(line));
    }

    expected_failure(CaseBuilder::default().finish("REQ", "control"));
    drop(control_success(
        control_case_builder().finish("REQ", "control"),
    ));
    let mut missing_scenario = control_case_builder();
    missing_scenario.scenario = None;
    expected_failure(missing_scenario.finish("REQ", "control"));
    let mut missing_polarity = control_case_builder();
    missing_polarity.polarity = None;
    expected_failure(missing_polarity.finish("REQ", "control"));
    let mut missing_operation = control_case_builder();
    missing_operation.operation = None;
    expected_failure(missing_operation.finish("REQ", "control"));
    let mut missing_baseline = control_case_builder();
    missing_baseline.baseline = None;
    expected_failure(missing_baseline.finish("REQ", "control"));
    let mut missing_outcome = control_case_builder();
    missing_outcome.outcome = None;
    expected_failure(missing_outcome.finish("REQ", "control"));
    let mut missing_mutations = control_case_builder();
    missing_mutations.mutations = None;
    expected_failure(missing_mutations.finish("REQ", "control"));
    let mut missing_codes = control_case_builder();
    missing_codes.codes = None;
    expected_failure(missing_codes.finish("REQ", "control"));
    let mut missing_facts = control_case_builder();
    missing_facts.facts = None;
    expected_failure(missing_facts.finish("REQ", "control"));

    let case_file = root.join("case.toml");
    let valid_case_source = format!(
        "# control\n\nschema = \"{CASE_SCHEMA}\"\nrequirement = \"SPG-CONTROL\"\n[[case]]\nid = \"control-case\"\nscenario = \"positive-control\"\npolarity = \"positive\"\noperation = \"control\"\nbaseline = \"control.toml\"\nmutations = []\nexpected = {{ outcome = \"accept\", codes = [], facts = [\"control=true\"] }}\n"
    );
    control_success(fs::write(&case_file, &valid_case_source).map_err(io_error));
    drop(control_success(parse_case_file(&case_file)));
    expected_failure(parse_case_file(&root.join("missing-case.toml")));
    for source in [
        valid_case_source.replacen("schema =", "unknown =", 1),
        valid_case_source.replacen("operation = \"control\"", "unknown = \"control\"", 1),
        valid_case_source.replacen(CASE_SCHEMA, "wrong-schema", 1),
        valid_case_source.replacen("requirement = \"SPG-CONTROL\"\n", "", 1),
        format!("schema = \"{CASE_SCHEMA}\"\nrequirement = \"SPG-CONTROL\"\n"),
        valid_case_source.replacen("mutations = []", "mutations = wrong", 1),
        valid_case_source.replacen("outcome = \"accept\"", "wrong = \"accept\"", 1),
        valid_case_source.replacen("codes = []", "codes = wrong", 1),
        valid_case_source.replacen("facts = [\"control=true\"]", "facts = wrong", 1),
        valid_case_source.replacen("id = \"control-case\"\n", "", 1),
        format!(
            "{}[[case]]\n",
            valid_case_source.replacen("id = \"control-case\"\n", "", 1)
        ),
    ] {
        control_success(fs::write(&case_file, source).map_err(io_error));
        expected_failure(parse_case_file(&case_file));
    }
    let second_case_source = valid_case_source.replace("control-case", "control-case-two");
    let (_, second_case_body) = second_case_source
        .split_once("[[case]]")
        .expect("control source contains a case");
    control_success(
        fs::write(
            &case_file,
            format!("{valid_case_source}[[case]]{second_case_body}"),
        )
        .map_err(io_error),
    );
    drop(control_success(parse_case_file(&case_file)));
    control_success(fs::write(&case_file, &valid_case_source).map_err(io_error));

    let case = control_case();
    control_success(validate_polarity(&case));
    for (polarity, outcome, codes, mutations, facts) in [
        ("unknown", "accept", vec![], vec![], vec!["fact"]),
        ("negative", "accept", vec![], vec![], vec![]),
        ("positive", "unknown", vec![], vec![], vec!["fact"]),
        ("positive", "accept", vec![], vec![], vec![]),
        ("negative", "accept", vec!["CODE"], vec![], vec!["fact"]),
        ("negative", "accept", vec![], vec!["change"], vec!["fact"]),
    ] {
        let mut changed = control_case();
        changed.polarity = polarity.to_owned();
        changed.outcome = outcome.to_owned();
        changed.codes = codes.into_iter().map(str::to_owned).collect();
        changed.mutations = mutations.into_iter().map(str::to_owned).collect();
        changed.facts = facts.into_iter().map(str::to_owned).collect();
        drop(validate_polarity(&changed));
    }

    let operations = BTreeSet::from([String::from("control")]);
    let mutations = BTreeSet::from([String::from("change")]);
    expected_failure(validate_case_collection(
        Vec::new(),
        &operations,
        &mutations,
    ));
    expected_failure(validate_case_collection(
        vec![case.clone(), case.clone()],
        &operations,
        &mutations,
    ));
    let mut unknown_operation = case.clone();
    unknown_operation.operation = String::from("unknown");
    expected_failure(validate_case_collection(
        vec![unknown_operation],
        &operations,
        &mutations,
    ));
    let mut invalid_collection_polarity = case.clone();
    invalid_collection_polarity.polarity = String::from("unknown");
    expected_failure(validate_case_collection(
        vec![invalid_collection_polarity],
        &operations,
        &mutations,
    ));
    let mut unknown_mutation = case.clone();
    unknown_mutation.mutations = vec![String::from("unknown")];
    expected_failure(validate_case_collection(
        vec![unknown_mutation],
        &operations,
        &mutations,
    ));
    drop(control_success(validate_case_collection(
        vec![case.clone()],
        &operations,
        &mutations,
    )));
    drop(control_success(read_cases_from_roots(
        &[root],
        &operations,
        &mutations,
    )));
    expected_failure(read_cases_from_roots(
        &[root.join("missing-case-root").as_path()],
        &operations,
        &mutations,
    ));
    let invalid_case_root = root.join("invalid-case-root");
    control_success(fs::create_dir_all(&invalid_case_root).map_err(io_error));
    control_success(
        fs::write(invalid_case_root.join("invalid.toml"), "invalid\n").map_err(io_error),
    );
    expected_failure(read_cases_from_roots(
        &[invalid_case_root.as_path()],
        &operations,
        &mutations,
    ));

    expected_failure(validate_baselines_from_root(
        &root.join("missing-baseline-root"),
        &[],
    ));
    let valid_baselines = root.join("valid-baselines");
    control_success(fs::create_dir_all(&valid_baselines).map_err(io_error));
    control_success(
        fs::write(
            valid_baselines.join("valid.toml"),
            format!("schema = \"{BASELINE_SCHEMA}\"\nid = \"baseline\"\n"),
        )
        .map_err(io_error),
    );
    control_success(validate_baselines_from_root(&valid_baselines, &[]));
    let invalid_baselines = root.join("invalid-baselines");
    control_success(fs::create_dir_all(&invalid_baselines).map_err(io_error));
    control_success(
        fs::write(
            invalid_baselines.join("invalid.toml"),
            "schema = \"wrong\"\n",
        )
        .map_err(io_error),
    );
    expected_failure(validate_baselines_from_root(&invalid_baselines, &[]));
    control_success(
        fs::write(
            invalid_baselines.join("invalid.toml"),
            "schema = \"wrong\"\nid = \"invalid\"\n",
        )
        .map_err(io_error),
    );
    expected_failure(validate_baselines_from_root(&invalid_baselines, &[]));
    control_success(fs::write(invalid_baselines.join("invalid.toml"), [0xff]).map_err(io_error));
    expected_failure(validate_baselines_from_root(&invalid_baselines, &[]));
    control_success(
        fs::write(
            invalid_baselines.join("invalid.toml"),
            format!("schema = \"{BASELINE_SCHEMA}\"\nid = \"invalid\"\nincludes = wrong\n"),
        )
        .map_err(io_error),
    );
    expected_failure(validate_baselines_from_root(&invalid_baselines, &[]));
    let duplicate_baselines = root.join("duplicate-baselines");
    control_success(fs::create_dir_all(&duplicate_baselines).map_err(io_error));
    for name in ["one.toml", "two.toml"] {
        control_success(
            fs::write(
                duplicate_baselines.join(name),
                format!("schema = \"{BASELINE_SCHEMA}\"\nid = \"duplicate\"\n"),
            )
            .map_err(io_error),
        );
    }
    expected_failure(validate_baselines_from_root(&duplicate_baselines, &[]));
    let include_baselines = root.join("include-baselines");
    control_success(fs::create_dir_all(&include_baselines).map_err(io_error));
    control_success(
        fs::write(
            include_baselines.join("include.toml"),
            format!(
                "schema = \"{BASELINE_SCHEMA}\"\nid = \"include\"\nincludes = [\"missing-include\"]\n"
            ),
        )
        .map_err(io_error),
    );
    expected_failure(validate_baselines_from_root(&include_baselines, &[]));
    let mut missing_baseline = case.clone();
    missing_baseline.baseline = String::from("missing-baseline");
    expected_failure(validate_baselines_from_root(
        &valid_baselines,
        &[missing_baseline],
    ));
    let mut directory_baseline = case.clone();
    directory_baseline.baseline = root.display().to_string();
    expected_failure(validate_baselines_from_root(
        &valid_baselines,
        &[directory_baseline.clone()],
    ));
    directory_baseline.operation = String::from("architecture-gate");
    expected_failure(validate_baselines_from_root(
        &valid_baselines,
        &[directory_baseline],
    ));

    let valid_specs = root.join("valid-specs");
    control_success(fs::create_dir_all(&valid_specs).map_err(io_error));
    control_success(
        fs::write(
            valid_specs.join("spec.md"),
            "### Requirement: SPG-CONTROL\n#### Scenario: Positive works\n#### Scenario: Negative fails\n",
        )
        .map_err(io_error),
    );
    drop(control_success(read_spec_scenarios_from_root(&valid_specs)));
    expected_failure(read_spec_scenarios_from_root(
        &root.join("missing-spec-root"),
    ));
    let invalid_spec_root = root.join("invalid-spec-root");
    control_success(fs::create_dir_all(&invalid_spec_root).map_err(io_error));
    control_success(fs::write(invalid_spec_root.join("spec.md"), [0xff]).map_err(io_error));
    expected_failure(read_spec_scenarios_from_root(&invalid_spec_root));
    for (directory, content) in [
        ("empty-specs", "no requirements\n"),
        ("early-scenario", "#### Scenario: Positive too early\n"),
        (
            "missing-polarity",
            "### Requirement: SPG-CONTROL\n#### Scenario: Maybe works\n",
        ),
        ("empty-requirement", "### Requirement: \n"),
    ] {
        let spec_root = root.join(directory);
        control_success(fs::create_dir_all(&spec_root).map_err(io_error));
        control_success(fs::write(spec_root.join("spec.md"), content).map_err(io_error));
        expected_failure(read_spec_scenarios_from_root(&spec_root));
    }

    let scenarios = BTreeMap::from([(
        String::from("SPG-CONTROL"),
        vec![SpecScenario {
            slug: String::from("positive-control"),
            match_key: scenario_match_key("positive-control"),
            polarity: String::from("positive"),
        }],
    )]);
    drop(control_success(render_manifest(
        std::slice::from_ref(&case),
        &scenarios,
    )));
    let mut unknown_requirement = case.clone();
    unknown_requirement.requirement = String::from("UNKNOWN");
    expected_failure(render_manifest(
        &[case.clone(), unknown_requirement],
        &scenarios,
    ));
    let mut no_match = case.clone();
    no_match.scenario = String::from("additional scenario");
    expected_failure(render_manifest(std::slice::from_ref(&no_match), &scenarios));
    let mut ambiguous = scenarios.clone();
    ambiguous
        .get_mut("SPG-CONTROL")
        .expect("control requirement exists")
        .push(SpecScenario {
            slug: String::from("positive-control"),
            match_key: scenario_match_key("positive-control"),
            polarity: String::from("positive"),
        });
    expected_failure(render_manifest(std::slice::from_ref(&case), &ambiguous));
    expected_failure(render_manifest(&[], &scenarios));

    let model_file = root.join("model.toml");
    let valid_model_source = format!(
        "# control\n\nschema = \"{MODEL_SCHEMA}\"\nid = \"model-control\"\nkind = \"transition-system\"\nrequirement_ids = [\"SPG-CONTROL\"]\nexpected_state_count = 1\nexpected_reachable_count = 1\ninitial = [\"live\"]\n[[dimensions]]\nname = \"value\"\nvalues = [\"a\"]\n[[predicates]]\nname = \"predicate\"\n[[predicates.clauses]]\nall = [\"value=a\"]\n[[states]]\nname = \"live\"\nterminal = false\nid_reserved = true\n[[transitions]]\nfrom = \"live\"\nevent = \"stay\"\nto = \"live\"\n[[invariants]]\nid = \"invariant\"\nrelation = \"all-states-reachable\"\nif_all = [\"value=a\"]\nthen_all = [\"predicate=true\"]\n"
    );
    control_success(fs::write(&model_file, &valid_model_source).map_err(io_error));
    drop(control_success(parse_model(&model_file)));
    expected_failure(parse_model(&root.join("missing-model.toml")));
    for source in [
        String::from("[[predicates.clauses]]\n"),
        valid_model_source.replacen(MODEL_SCHEMA, "wrong-schema", 1),
        valid_model_source.replacen("id = \"model-control\"\n", "", 1),
        valid_model_source.replacen("kind = \"transition-system\"", "unknown = \"x\"", 1),
    ] {
        control_success(fs::write(&model_file, source).map_err(io_error));
        expected_failure(parse_model(&model_file));
    }
    control_success(fs::write(&model_file, &valid_model_source).map_err(io_error));

    let mut field_model = control_truth_model();
    field_model.states.push(ModelState::default());
    field_model.transitions.push(Transition::default());
    for section in [
        ModelSection::Top,
        ModelSection::Dimension(0),
        ModelSection::Predicate(0),
        ModelSection::Clause(0, 0),
        ModelSection::State(0),
        ModelSection::Transition(0),
        ModelSection::Invariant(0),
    ] {
        let mut schema = None;
        expected_failure(parse_model_field(
            &model_file,
            1,
            "unknown = \"x\"",
            section,
            &mut field_model,
            &mut schema,
        ));
    }
    for (section, line) in [
        (ModelSection::Top, "requirement_ids = wrong"),
        (ModelSection::Top, "initial = wrong"),
        (ModelSection::Top, "expected_state_count = wrong"),
        (ModelSection::Top, "expected_reachable_count = wrong"),
        (ModelSection::Dimension(0), "values = wrong"),
        (ModelSection::Clause(0, 0), "all = wrong"),
        (ModelSection::State(0), "terminal = wrong"),
        (ModelSection::State(0), "id_reserved = wrong"),
        (ModelSection::Invariant(0), "if_all = wrong"),
        (ModelSection::Invariant(0), "then_all = wrong"),
    ] {
        let mut schema = None;
        expected_failure(parse_model_field(
            &model_file,
            1,
            line,
            section,
            &mut field_model,
            &mut schema,
        ));
    }

    let expected_control_models = BTreeSet::from([String::from("truth-control")]);
    drop(control_success(validate_model_set(
        vec![(PathBuf::from("truth-control.toml"), control_truth_model())],
        &expected_control_models,
    )));
    expected_failure(validate_model_set(
        vec![
            (PathBuf::from("one.toml"), control_truth_model()),
            (PathBuf::from("two.toml"), control_truth_model()),
        ],
        &expected_control_models,
    ));
    let mut invalid_truth_set = control_truth_model();
    invalid_truth_set.dimensions.clear();
    expected_failure(validate_model_set(
        vec![(PathBuf::from("invalid-truth.toml"), invalid_truth_set)],
        &expected_control_models,
    ));
    let mut invalid_transition_set = control_transition_model();
    invalid_transition_set.states.clear();
    let expected_transition_models = BTreeSet::from([String::from("transition-control")]);
    expected_failure(validate_model_set(
        vec![(
            PathBuf::from("invalid-transition.toml"),
            invalid_transition_set,
        )],
        &expected_transition_models,
    ));
    let mut unknown_kind = control_truth_model();
    unknown_kind.kind = String::from("unknown");
    expected_failure(validate_model_set(
        vec![(PathBuf::from("unknown.toml"), unknown_kind)],
        &expected_control_models,
    ));
    expected_failure(validate_model_set(Vec::new(), &expected_control_models));
    expected_failure(validate_models_from_root(
        &root.join("missing-model-root"),
        &expected_control_models,
    ));
    let valid_model_root = root.join("valid-model-root");
    control_success(fs::create_dir_all(&valid_model_root).map_err(io_error));
    control_success(
        fs::write(valid_model_root.join("model.toml"), &valid_model_source).map_err(io_error),
    );
    let parsed_model_ids = BTreeSet::from([String::from("model-control")]);
    drop(control_success(validate_models_from_root(
        &valid_model_root,
        &parsed_model_ids,
    )));
    control_success(fs::write(valid_model_root.join("model.toml"), "invalid\n").map_err(io_error));
    expected_failure(validate_models_from_root(
        &valid_model_root,
        &parsed_model_ids,
    ));

    let _ = control_success(validate_truth_model(&control_truth_model()));
    let mut truth_controls = Vec::new();
    truth_controls.push(Model::default());
    let mut invalid_dimension = control_truth_model();
    invalid_dimension.dimensions[0].0.clear();
    truth_controls.push(invalid_dimension);
    let mut empty_values = control_truth_model();
    empty_values.dimensions[0].1.clear();
    truth_controls.push(empty_values);
    let mut duplicate_dimension = control_truth_model();
    duplicate_dimension
        .dimensions
        .push(duplicate_dimension.dimensions[0].clone());
    truth_controls.push(duplicate_dimension);
    let mut duplicate_value = control_truth_model();
    duplicate_value.dimensions[0].1 = vec![String::from("a"), String::from("a")];
    truth_controls.push(duplicate_value);
    let mut large_count = control_truth_model();
    large_count.dimensions = (0..17)
        .map(|index| {
            (
                format!("dimension-{index}"),
                vec![String::from("a"), String::from("b")],
            )
        })
        .collect();
    truth_controls.push(large_count);
    let mut overflow_count = control_truth_model();
    overflow_count.dimensions = (0..usize::BITS)
        .map(|index| {
            (
                format!("dimension-{index}"),
                vec![String::from("a"), String::from("b")],
            )
        })
        .collect();
    truth_controls.push(overflow_count);
    let mut wrong_count = control_truth_model();
    wrong_count.expected_state_count = Some(3);
    truth_controls.push(wrong_count);
    let mut invalid_predicate = control_truth_model();
    invalid_predicate.predicates[0].clauses.clear();
    truth_controls.push(invalid_predicate);
    let mut unnamed_predicate = control_truth_model();
    unnamed_predicate.predicates[0].name.clear();
    truth_controls.push(unnamed_predicate);
    let mut duplicate_predicate = control_truth_model();
    duplicate_predicate.predicates.push(Predicate {
        name: String::from("is-a"),
        clauses: vec![vec![String::from("value=a")]],
    });
    truth_controls.push(duplicate_predicate);
    let mut invalid_invariant = control_truth_model();
    invalid_invariant.invariants[0].relation = Some(String::from("wrong"));
    truth_controls.push(invalid_invariant);
    let mut unnamed_invariant = control_truth_model();
    unnamed_invariant.invariants[0].id.clear();
    truth_controls.push(unnamed_invariant);
    let mut duplicate_invariant = control_truth_model();
    duplicate_invariant.invariants.push(ModelInvariant {
        id: String::from("truth-invariant"),
        ..ModelInvariant::default()
    });
    truth_controls.push(duplicate_invariant);
    let mut malformed_condition = control_truth_model();
    malformed_condition.predicates[0].clauses[0][0] = String::from("unknown=value");
    truth_controls.push(malformed_condition);
    let mut malformed_invariant_if = control_truth_model();
    malformed_invariant_if.invariants[0].if_all = vec![String::from("unknown=value")];
    truth_controls.push(malformed_invariant_if);
    let mut malformed_invariant_then = control_truth_model();
    malformed_invariant_then.invariants[0].then_all = vec![String::from("unknown=value")];
    truth_controls.push(malformed_invariant_then);
    let mut failed_invariant = control_truth_model();
    failed_invariant.invariants[0].then_all = vec![String::from("is-a=false")];
    truth_controls.push(failed_invariant);
    for model in &truth_controls {
        expected_failure(validate_truth_model(model));
    }

    let _ = control_success(validate_transition_model(&control_transition_model()));
    let mut transition_controls = Vec::new();
    transition_controls.push(Model::default());
    let mut invalid_state = control_transition_model();
    invalid_state.states[0].name.clear();
    transition_controls.push(invalid_state);
    let mut duplicate_state = control_transition_model();
    duplicate_state.states.push(ModelState {
        name: String::from("live"),
        terminal: false,
        id_reserved: true,
    });
    transition_controls.push(duplicate_state);
    let mut unknown_initial = control_transition_model();
    unknown_initial.initial[0] = String::from("unknown");
    transition_controls.push(unknown_initial);
    let mut unknown_transition = control_transition_model();
    unknown_transition.transitions[0].to = String::from("unknown");
    transition_controls.push(unknown_transition);
    let mut duplicate_event = control_transition_model();
    duplicate_event.transitions.push(Transition {
        from: String::from("live"),
        event: String::from("stop"),
        to: String::from("dead"),
    });
    transition_controls.push(duplicate_event);
    let mut empty_event = control_transition_model();
    empty_event.transitions[0].event.clear();
    transition_controls.push(empty_event);
    let mut terminal_transition = control_transition_model();
    terminal_transition.transitions[0].from = String::from("dead");
    transition_controls.push(terminal_transition);
    let mut wrong_reachable = control_transition_model();
    wrong_reachable.expected_reachable_count = Some(1);
    transition_controls.push(wrong_reachable);
    let mut unreachable_state = control_transition_model();
    unreachable_state.transitions.clear();
    unreachable_state.expected_reachable_count = Some(1);
    transition_controls.push(unreachable_state);
    let mut missing_relation = control_transition_model();
    missing_relation.invariants[0].relation = None;
    transition_controls.push(missing_relation);
    let mut duplicate_transition_invariant = control_transition_model();
    duplicate_transition_invariant
        .invariants
        .push(ModelInvariant {
            id: String::from("reachable"),
            relation: Some(String::from("all-states-reachable")),
            ..ModelInvariant::default()
        });
    transition_controls.push(duplicate_transition_invariant);
    let mut released_id = control_transition_model();
    released_id.states[0].id_reserved = false;
    transition_controls.push(released_id);
    let mut unknown_relation = control_transition_model();
    unknown_relation.invariants[0].relation = Some(String::from("unknown"));
    transition_controls.push(unknown_relation);
    for model in &transition_controls {
        expected_failure(validate_transition_model(model));
    }

    let state = BTreeMap::from([(String::from("value"), String::from("a"))]);
    let predicates = BTreeMap::from([(String::from("predicate"), true)]);
    for condition in [
        "value=a",
        "value!=b",
        "predicate=true",
        "predicate=false",
        "predicate=wrong",
        "unknown=value",
        "malformed",
    ] {
        let _ = eval_condition(condition, &state, &predicates);
        drop(validate_condition(
            condition,
            &state,
            &predicates,
            "control",
        ));
    }
    drop(state_for_ordinal(
        &[(
            String::from("value"),
            vec![String::from("a"), String::from("b")],
        )],
        1,
    ));

    for (line, key) in [
        ("name = \"value\"", "name"),
        ("wrong", "name"),
        ("name = \"unterminated", "name"),
    ] {
        drop(string_assignment(line, key));
    }
    for line in ["count = 1", "wrong", "count = invalid"] {
        drop(usize_assignment(line, "count"));
    }
    for line in ["flag = true", "flag = false", "wrong", "flag = invalid"] {
        drop(bool_assignment(line, "flag"));
    }
    for text in [
        "[]",
        "[\"a\", \"b\"]",
        "wrong",
        "[1]",
        "[\"a\\b\"]",
        "[\"unterminated]",
        "[ , ]",
    ] {
        drop(parse_quoted_array(text));
    }
    drop(array_assignment("values = [\"a\"]", "values"));
    drop(array_assignment("wrong", "values"));
    for line in [
        "expected = { outcome = \"accept\", codes = [], facts = [] }",
        "wrong",
        "expected = { outcome = \"unterminated }",
        "expected = { outcome = \"accept\", codes = [",
    ] {
        drop(inline_string(line, "outcome"));
        drop(inline_array(line, "codes"));
    }
    drop(find_top_string(
        "id = \"control\"\n[section]\nid = \"ignored\"",
        "id",
    ));
    drop(find_top_array("includes = [\"a\"]\n[section]", "includes"));
    drop(find_top_array("id = \"control\"\n[section]", "includes"));
    let mut extension_paths = Vec::new();
    expected_failure(collect_extension(
        &root.join("missing-extension-root"),
        "toml",
        &mut extension_paths,
    ));
    let mut failed_entries = [Err(std::io::Error::other("control entry failure"))].into_iter();
    expected_failure(collect_extension_entries(
        root,
        "toml",
        &mut extension_paths,
        &mut failed_entries,
    ));
    control_success(collect_extension(root, "not-present", &mut extension_paths));
    drop(slash_path(Path::new("a\\b")));
    for input in ["Positive, control!", "--", "-a", "Already-Slug"] {
        drop(slugify(input));
    }
    drop(scenario_match_key("the-positive-control-is-valid"));

    println!("fixture-manifest self-test: ok");
    Ok(())
}

enum Action {
    Validate(bool),
    SelfTest,
}

fn parse_arguments(arguments: &[String]) -> CheckResult<Action> {
    match arguments {
        [] => Ok(Action::Validate(false)),
        [argument] => match argument.as_str() {
            "--write-manifest" => Ok(Action::Validate(true)),
            "--self-test" => Ok(Action::SelfTest),
            _ => Err(String::from(
                "usage: check-fixture-manifest [--write-manifest|--self-test]",
            )),
        },
        _ => Err(String::from(
            "usage: check-fixture-manifest [--write-manifest|--self-test]",
        )),
    }
}

fn run_arguments(arguments: &[String]) -> CheckResult<()> {
    parse_arguments(arguments).and_then(|action| match action {
        Action::Validate(write_manifest) => validate_repository(write_manifest),
        Action::SelfTest => self_test(),
    })
}

fn parse_vocabularies(path: &Path) -> CheckResult<(BTreeSet<String>, BTreeSet<String>)> {
    let text =
        fs::read_to_string(path).map_err(|error| format!("read {}: {error}", path.display()))?;
    let mut operations = BTreeSet::new();
    let mut mutations = BTreeSet::new();
    let mut in_operations = false;
    let mut in_mutations = false;

    for line in text.lines() {
        match line.trim() {
            "## SPG operations" => {
                in_operations = true;
                in_mutations = false;
                continue;
            }
            "## SPG mutations" => {
                in_operations = false;
                in_mutations = true;
                continue;
            }
            "## Expected facts" => {
                in_operations = false;
                in_mutations = false;
                continue;
            }
            _ => {}
        }
        let Some(name) = backtick_bullet(line) else {
            continue;
        };
        if in_operations {
            operations.insert(name);
        } else if in_mutations {
            mutations.insert(name);
        }
    }

    if [operations.is_empty(), mutations.is_empty()].contains(&true) {
        return Err(format!(
            "{} must declare nonempty operation and mutation vocabularies",
            path.display()
        ));
    }
    Ok((operations, mutations))
}

fn backtick_bullet(line: &str) -> Option<String> {
    let trimmed = line.trim();
    let rest = trimmed.strip_prefix("- `")?;
    let end = rest.find('`')?;
    let candidate = &rest[..end];
    if [
        candidate.contains('<'),
        candidate.contains('='),
        candidate.contains(':'),
    ]
    .contains(&true)
    {
        return None;
    }
    Some(candidate.to_owned())
}

fn read_cases(
    operations: &BTreeSet<String>,
    mutations: &BTreeSet<String>,
) -> CheckResult<Vec<Case>> {
    let roots = CASE_ROOTS.map(Path::new);
    read_cases_from_roots(&roots, operations, mutations)
}

fn read_cases_from_roots(
    roots: &[&Path],
    operations: &BTreeSet<String>,
    mutations: &BTreeSet<String>,
) -> CheckResult<Vec<Case>> {
    let mut paths = Vec::new();
    for root in roots {
        collect_extension(root, "toml", &mut paths)?;
    }
    paths.sort();
    let mut all = Vec::new();
    for path in paths {
        all.append(&mut parse_case_file(&path)?);
    }
    validate_case_collection(all, operations, mutations)
}

fn validate_case_collection(
    mut cases: Vec<Case>,
    operations: &BTreeSet<String>,
    mutations: &BTreeSet<String>,
) -> CheckResult<Vec<Case>> {
    if cases.is_empty() {
        return Err("no fixture case files found".to_owned());
    }
    let mut ids = BTreeSet::new();
    for case in &cases {
        if !ids.insert(case.id.clone()) {
            return Err(format!("duplicate fixture case ID `{}`", case.id));
        }
        if !operations.contains(&case.operation) {
            return Err(format!(
                "{}: unknown operation `{}`",
                case.id, case.operation
            ));
        }
        for mutation in &case.mutations {
            if !mutations.contains(mutation) {
                return Err(format!("{}: unknown mutation `{mutation}`", case.id));
            }
        }
        validate_polarity(case)?;
    }
    cases.sort_by(|left, right| left.id.cmp(&right.id));
    Ok(cases)
}

fn parse_case_file(path: &Path) -> CheckResult<Vec<Case>> {
    let text =
        fs::read_to_string(path).map_err(|error| format!("read {}: {error}", path.display()))?;
    let fixture = slash_path(path);
    let mut schema = None;
    let mut requirement = None;
    let mut current: Option<CaseBuilder> = None;
    let mut result = Vec::new();

    for (index, raw) in text.lines().enumerate() {
        let line = raw.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        if line == "[[case]]" {
            if let Some(builder) = current.take() {
                result.push(builder.finish(requirement.as_deref().unwrap_or(""), &fixture)?);
            }
            current = Some(CaseBuilder::default());
            continue;
        }
        if let Some(builder) = current.as_mut() {
            if let Some(value) = string_assignment(line, "id") {
                builder.id = Some(value);
            } else if let Some(value) = string_assignment(line, "scenario") {
                builder.scenario = Some(value);
            } else if let Some(value) = string_assignment(line, "polarity") {
                builder.polarity = Some(value);
            } else if let Some(value) = string_assignment(line, "operation") {
                builder.operation = Some(value);
            } else if let Some(value) = string_assignment(line, "baseline") {
                builder.baseline = Some(value);
            } else if line.starts_with("mutations = ") {
                builder.mutations = Some(array_assignment(line, "mutations")?);
            } else if line.starts_with("expected = ") {
                builder.outcome = Some(inline_string(line, "outcome")?);
                builder.codes = Some(inline_array(line, "codes")?);
                builder.facts = Some(inline_array(line, "facts")?);
            } else {
                return Err(format!(
                    "{}:{}: unknown case field `{line}`",
                    path.display(),
                    index + 1
                ));
            }
        } else if let Some(value) = string_assignment(line, "schema") {
            schema = Some(value);
        } else if let Some(value) = string_assignment(line, "requirement") {
            requirement = Some(value);
        } else {
            return Err(format!(
                "{}:{}: unknown top-level field `{line}`",
                path.display(),
                index + 1
            ));
        }
    }
    if let Some(builder) = current {
        result.push(builder.finish(requirement.as_deref().unwrap_or(""), &fixture)?);
    }
    if schema.as_deref() != Some(CASE_SCHEMA) {
        return Err(format!("{fixture}: expected schema `{CASE_SCHEMA}`"));
    }
    requirement.ok_or_else(|| format!("{fixture}: missing requirement"))?;
    if result.is_empty() {
        return Err(format!("{fixture}: no cases"));
    }
    Ok(result)
}

fn validate_polarity(case: &Case) -> CheckResult<()> {
    match case.polarity.as_str() {
        "positive" => {}
        "negative" => {
            let has_mutated_fact_witness =
                [!case.mutations.is_empty(), !case.facts.is_empty()].contains(&false) == false;
            if [
                case.outcome != "reject",
                case.codes.is_empty(),
                !has_mutated_fact_witness,
            ]
            .iter()
            .all(|condition| *condition)
            {
                return Err(format!(
                    "{}: negative case has no rejection, diagnostic, or mutated fact witness",
                    case.id
                ));
            }
        }
        other => return Err(format!("{}: invalid polarity `{other}`", case.id)),
    }
    if !["accept", "reject"].contains(&case.outcome.as_str()) {
        return Err(format!(
            "{}: invalid expected outcome `{}`",
            case.id, case.outcome
        ));
    }
    if case.facts.is_empty() {
        return Err(format!("{}: expected facts must be nonempty", case.id));
    }
    Ok(())
}

fn validate_baselines(cases: &[Case]) -> CheckResult<()> {
    validate_baselines_from_root(Path::new(BASELINE_ROOT), cases)
}

fn validate_baselines_from_root(root: &Path, cases: &[Case]) -> CheckResult<()> {
    let mut baseline_paths = Vec::new();
    collect_extension(root, "toml", &mut baseline_paths)?;
    let mut baseline_ids = BTreeSet::new();
    for path in baseline_paths {
        let text = fs::read_to_string(&path)
            .map_err(|error| format!("read {}: {error}", path.display()))?;
        let schema = find_top_string(&text, "schema");
        let id = find_top_string(&text, "id")
            .ok_or_else(|| format!("{}: missing baseline ID", path.display()))?;
        if schema.as_deref() != Some(BASELINE_SCHEMA) {
            return Err(format!(
                "{}: expected schema `{BASELINE_SCHEMA}`",
                path.display()
            ));
        }
        if !baseline_ids.insert(id.clone()) {
            return Err(format!("duplicate baseline ID `{id}`"));
        }
        if let Some(includes) = find_top_array(&text, "includes")? {
            for include in includes {
                if !Path::new(&include).is_file() {
                    return Err(format!("{}: missing include `{include}`", path.display()));
                }
            }
        }
    }

    for case in cases {
        let path = Path::new(&case.baseline);
        if !path.exists() {
            return Err(format!("{}: missing baseline `{}`", case.id, case.baseline));
        }
        if case.operation != "architecture-gate" && !path.is_file() {
            return Err(format!("{}: baseline must be a file", case.id));
        }
        if case.operation == "architecture-gate" && !path.starts_with("fixtures/dylint") {
            return Err(format!(
                "{}: architecture baseline is outside fixtures/dylint",
                case.id
            ));
        }
    }
    Ok(())
}

fn read_spec_scenarios() -> CheckResult<BTreeMap<String, Vec<SpecScenario>>> {
    read_spec_scenarios_from_root(Path::new(SPEC_ROOT))
}

fn read_spec_scenarios_from_root(root: &Path) -> CheckResult<BTreeMap<String, Vec<SpecScenario>>> {
    let mut paths = Vec::new();
    collect_extension(root, "md", &mut paths)?;
    paths.sort();
    let mut result: BTreeMap<String, Vec<SpecScenario>> = BTreeMap::new();
    for path in paths {
        let text = fs::read_to_string(&path)
            .map_err(|error| format!("read {}: {error}", path.display()))?;
        let mut requirement: Option<String> = None;
        for line in text.lines() {
            if let Some(rest) = line.strip_prefix("### Requirement: ") {
                let id = rest
                    .split_whitespace()
                    .next()
                    .ok_or_else(|| format!("{}: empty requirement heading", path.display()))?;
                requirement = Some(id.to_owned());
                result.entry(id.to_owned()).or_default();
            } else if let Some(rest) = line.strip_prefix("#### Scenario: ") {
                let req = requirement
                    .as_ref()
                    .ok_or_else(|| format!("{}: scenario before requirement", path.display()))?;
                let slug = slugify(rest);
                let polarity = if rest.trim_start().starts_with("Positive") {
                    "positive"
                } else if rest.trim_start().starts_with("Negative") {
                    "negative"
                } else {
                    return Err(format!(
                        "{}: scenario lacks polarity: {rest}",
                        path.display()
                    ));
                };
                result.entry(req.clone()).or_default().push(SpecScenario {
                    match_key: scenario_match_key(&slug),
                    slug,
                    polarity: polarity.to_owned(),
                });
            }
        }
    }
    if result.is_empty() {
        return Err("no OpenSpec requirements found".to_owned());
    }
    Ok(result)
}

fn render_manifest(
    cases: &[Case],
    scenarios: &BTreeMap<String, Vec<SpecScenario>>,
) -> CheckResult<String> {
    let mut rows = Vec::new();
    let mut covered: BTreeMap<(String, String), usize> = BTreeMap::new();
    let case_requirements: BTreeSet<&str> =
        cases.iter().map(|case| case.requirement.as_str()).collect();
    for requirement in scenarios.keys() {
        if !case_requirements.contains(requirement.as_str()) {
            return Err(format!("requirement `{requirement}` has no fixture cases"));
        }
    }
    for case in cases {
        let declared = scenarios
            .get(&case.requirement)
            .ok_or_else(|| format!("{}: unknown requirement `{}`", case.id, case.requirement))?;
        let case_key = scenario_match_key(&case.scenario);
        let matches: Vec<_> = declared
            .iter()
            .filter(|scenario| scenario.polarity == case.polarity && scenario.match_key == case_key)
            .collect();
        let spec_scenario = match matches.as_slice() {
            [] => format!("additional/{}", case.scenario),
            [scenario] => {
                *covered
                    .entry((case.requirement.clone(), scenario.slug.clone()))
                    .or_default() += 1;
                scenario.slug.clone()
            }
            _ => return Err(format!("{}: ambiguous OpenSpec scenario match", case.id)),
        };
        rows.push(format!(
            "{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}",
            case.requirement,
            spec_scenario,
            case.polarity,
            case.fixture,
            case.id,
            case.scenario,
            case.operation,
            FINAL_ORACLE
        ));
    }
    for (requirement, declared) in scenarios {
        for scenario in declared {
            let count = covered
                .get(&(requirement.clone(), scenario.slug.clone()))
                .copied()
                .unwrap_or(0);
            if count != 1 {
                return Err(format!(
                    "OpenSpec scenario `{requirement}/{}` has {count} matching fixture cases; expected exactly one",
                    scenario.slug
                ));
            }
        }
    }
    rows.sort();
    let mut output = String::from(
        "requirement\tspec_scenario\tpolarity\tfixture\tcase_id\tfixture_scenario\tfocused_slice\tfinal_oracle\n",
    );
    output.push_str(&rows.join("\n"));
    output.push('\n');
    Ok(output)
}

fn validate_models() -> CheckResult<Vec<ModelResult>> {
    const EXPECTED_MODELS: [&str; 21] = [
        "model-artifact-index-freshness-v2",
        "model-atom-lifecycle-v2",
        "model-carrier-derivation-freshness-v2",
        "model-chelis-impact-closure-v1",
        "model-chelis-no-laundering-v1",
        "model-chelis-policy-adoption-v1",
        "model-chelis-proof-invalidation-v1",
        "model-chelis-required-role-coverage-v1",
        "model-chelis-surface-completeness-v1",
        "model-change-impact-closure-v2",
        "model-complete-impact-fixed-point-v2",
        "model-coverage-policy-adoption-v2",
        "model-debt-waiver-reaffirmation-no-laundering-v2",
        "model-decision-surface-completeness-v2",
        "model-governance-no-laundering-hardening-v2",
        "model-policy-weakening-protocol-v2",
        "model-proof-invalidation-hardening-v2",
        "model-proof-invalidation-v2",
        "model-required-role-coverage-v2",
        "model-terminal-governance-history-v2",
        "model-verdict-aggregation-v2",
    ];

    let expected = EXPECTED_MODELS.into_iter().map(str::to_owned).collect();
    let roots = MODEL_ROOTS.map(Path::new);
    validate_models_from_roots(&roots, &expected)
}

fn validate_models_from_roots(
    roots: &[&Path],
    expected: &BTreeSet<String>,
) -> CheckResult<Vec<ModelResult>> {
    let mut paths = Vec::new();
    for root in roots {
        collect_extension(root, "toml", &mut paths)?;
    }
    paths.sort();
    let models = paths
        .iter()
        .map(|path| parse_model(path).map(|model| (path.clone(), model)))
        .collect::<CheckResult<Vec<_>>>()?;
    validate_model_set(models, expected)
}

fn validate_models_from_root(
    root: &Path,
    expected: &BTreeSet<String>,
) -> CheckResult<Vec<ModelResult>> {
    validate_models_from_roots(&[root], expected)
}

fn validate_model_set(
    models: Vec<(PathBuf, Model)>,
    expected: &BTreeSet<String>,
) -> CheckResult<Vec<ModelResult>> {
    let mut ids = BTreeSet::new();
    let mut results = Vec::new();
    for (path, model) in models {
        if !ids.insert(model.id.clone()) {
            return Err(format!("duplicate model ID `{}`", model.id));
        }
        let state_count = match model.kind.as_str() {
            "truth-table" => validate_truth_model(&model)?,
            "transition-system" => validate_transition_model(&model)?,
            other => return Err(format!("{}: unknown model kind `{other}`", path.display())),
        };
        results.push(ModelResult {
            id: model.id,
            kind: model.kind,
            state_count,
            invariant_count: model.invariants.len(),
        });
    }
    if ids != *expected {
        let missing: Vec<_> = expected.difference(&ids).cloned().collect();
        let unexpected: Vec<_> = ids.difference(expected).cloned().collect();
        return Err(format!(
            "reference model set mismatch; missing={missing:?}, unexpected={unexpected:?}"
        ));
    }
    results.sort_by(|left, right| left.id.cmp(&right.id));
    Ok(results)
}

fn parse_model(path: &Path) -> CheckResult<Model> {
    let text =
        fs::read_to_string(path).map_err(|error| format!("read {}: {error}", path.display()))?;
    let mut model = Model::default();
    let mut schema = None;
    let mut section = ModelSection::Top;

    for (index, raw) in text.lines().enumerate() {
        let line = raw.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        section = match line {
            "[[dimensions]]" => {
                model.dimensions.push((String::new(), Vec::new()));
                ModelSection::Dimension(model.dimensions.len() - 1)
            }
            "[[predicates]]" => {
                model.predicates.push(Predicate::default());
                ModelSection::Predicate(model.predicates.len() - 1)
            }
            "[[predicates.clauses]]" => {
                let predicate = model
                    .predicates
                    .len()
                    .checked_sub(1)
                    .ok_or_else(|| format!("{}: clause before predicate", path.display()))?;
                model.predicates[predicate].clauses.push(Vec::new());
                ModelSection::Clause(predicate, model.predicates[predicate].clauses.len() - 1)
            }
            "[[states]]" => {
                model.states.push(ModelState::default());
                ModelSection::State(model.states.len() - 1)
            }
            "[[transitions]]" => {
                model.transitions.push(Transition::default());
                ModelSection::Transition(model.transitions.len() - 1)
            }
            "[[invariants]]" => {
                model.invariants.push(ModelInvariant::default());
                ModelSection::Invariant(model.invariants.len() - 1)
            }
            _ => {
                parse_model_field(path, index + 1, line, section, &mut model, &mut schema)?;
                section
            }
        };
    }
    if schema.as_deref() != Some(MODEL_SCHEMA) {
        return Err(format!(
            "{}: expected schema `{MODEL_SCHEMA}`",
            path.display()
        ));
    }
    if [model.id.is_empty(), model.requirement_ids.is_empty()].contains(&true) {
        return Err(format!(
            "{}: model ID and requirements are mandatory",
            path.display()
        ));
    }
    Ok(model)
}

fn parse_model_field(
    path: &Path,
    line_number: usize,
    line: &str,
    section: ModelSection,
    model: &mut Model,
    schema: &mut Option<String>,
) -> CheckResult<()> {
    match section {
        ModelSection::Top => {
            if let Some(value) = string_assignment(line, "schema") {
                *schema = Some(value);
            } else if let Some(value) = string_assignment(line, "id") {
                model.id = value;
            } else if let Some(value) = string_assignment(line, "kind") {
                model.kind = value;
            } else if line.starts_with("requirement_ids = ") {
                model.requirement_ids = array_assignment(line, "requirement_ids")?;
            } else if line.starts_with("initial = ") {
                model.initial = array_assignment(line, "initial")?;
            } else if let Some(value) = usize_assignment(line, "expected_state_count")? {
                model.expected_state_count = Some(value);
            } else if let Some(value) = usize_assignment(line, "expected_reachable_count")? {
                model.expected_reachable_count = Some(value);
            } else {
                return unknown_model_field(path, line_number, line);
            }
        }
        ModelSection::Dimension(slot) => {
            if let Some(value) = string_assignment(line, "name") {
                model.dimensions[slot].0 = value;
            } else if line.starts_with("values = ") {
                model.dimensions[slot].1 = array_assignment(line, "values")?;
            } else {
                return unknown_model_field(path, line_number, line);
            }
        }
        ModelSection::Predicate(slot) => {
            if let Some(value) = string_assignment(line, "name") {
                model.predicates[slot].name = value;
            } else {
                return unknown_model_field(path, line_number, line);
            }
        }
        ModelSection::Clause(predicate, clause) => {
            if line.starts_with("all = ") {
                model.predicates[predicate].clauses[clause] = array_assignment(line, "all")?;
            } else {
                return unknown_model_field(path, line_number, line);
            }
        }
        ModelSection::State(slot) => {
            if let Some(value) = string_assignment(line, "name") {
                model.states[slot].name = value;
            } else if let Some(value) = bool_assignment(line, "terminal")? {
                model.states[slot].terminal = value;
            } else if let Some(value) = bool_assignment(line, "id_reserved")? {
                model.states[slot].id_reserved = value;
            } else {
                return unknown_model_field(path, line_number, line);
            }
        }
        ModelSection::Transition(slot) => {
            if let Some(value) = string_assignment(line, "from") {
                model.transitions[slot].from = value;
            } else if let Some(value) = string_assignment(line, "event") {
                model.transitions[slot].event = value;
            } else if let Some(value) = string_assignment(line, "to") {
                model.transitions[slot].to = value;
            } else {
                return unknown_model_field(path, line_number, line);
            }
        }
        ModelSection::Invariant(slot) => {
            if let Some(value) = string_assignment(line, "id") {
                model.invariants[slot].id = value;
            } else if let Some(value) = string_assignment(line, "relation") {
                model.invariants[slot].relation = Some(value);
            } else if line.starts_with("if_all = ") {
                model.invariants[slot].if_all = array_assignment(line, "if_all")?;
            } else if line.starts_with("then_all = ") {
                model.invariants[slot].then_all = array_assignment(line, "then_all")?;
            } else {
                return unknown_model_field(path, line_number, line);
            }
        }
    }
    Ok(())
}

fn unknown_model_field(path: &Path, line_number: usize, line: &str) -> CheckResult<()> {
    Err(format!(
        "{}:{line_number}: unknown model field `{line}`",
        path.display()
    ))
}

fn validate_truth_model(model: &Model) -> CheckResult<usize> {
    if [
        model.dimensions.is_empty(),
        model.predicates.is_empty(),
        model.invariants.is_empty(),
    ]
    .contains(&true)
    {
        return Err(format!("{}: truth model is incomplete", model.id));
    }
    let mut dimension_names = BTreeSet::new();
    let mut count = 1usize;
    for (name, values) in &model.dimensions {
        if [
            name.is_empty(),
            values.is_empty(),
            !dimension_names.insert(name.clone()),
        ]
        .contains(&true)
        {
            return Err(format!(
                "{}: invalid or duplicate dimension `{name}`",
                model.id
            ));
        }
        let unique: BTreeSet<_> = values.iter().collect();
        if unique.len() != values.len() {
            return Err(format!(
                "{}: duplicate value in dimension `{name}`",
                model.id
            ));
        }
        count = count
            .checked_mul(values.len())
            .ok_or_else(|| format!("{}: state count overflow", model.id))?;
    }
    if [count > 100_000, model.expected_state_count != Some(count)].contains(&true) {
        return Err(format!(
            "{}: expected_state_count {:?} does not equal product {count}",
            model.id, model.expected_state_count
        ));
    }
    let mut predicate_names = BTreeSet::new();
    for predicate in &model.predicates {
        if [
            predicate.name.is_empty(),
            predicate.clauses.is_empty(),
            !predicate_names.insert(predicate.name.clone()),
        ]
        .contains(&true)
        {
            return Err(format!(
                "{}: invalid predicate `{}`",
                model.id, predicate.name
            ));
        }
    }
    let mut invariant_ids = BTreeSet::new();
    for invariant in &model.invariants {
        if [
            invariant.id.is_empty(),
            !invariant_ids.insert(invariant.id.clone()),
        ]
        .contains(&true)
        {
            return Err(format!(
                "{}: invalid invariant ID `{}`",
                model.id, invariant.id
            ));
        }
        if invariant.relation.is_some() {
            return Err(format!("{}: truth invariant cannot use relation", model.id));
        }
    }

    for ordinal in 0..count {
        let state = state_for_ordinal(&model.dimensions, ordinal);
        let mut predicates = BTreeMap::new();
        for predicate in &model.predicates {
            let value = predicate.clauses.iter().any(|clause| {
                clause.iter().all(|condition| {
                    eval_condition(condition, &state, &predicates).unwrap_or(false)
                })
            });
            for clause in &predicate.clauses {
                for condition in clause {
                    validate_condition(condition, &state, &predicates, &model.id)?;
                }
            }
            predicates.insert(predicate.name.clone(), value);
        }
        for invariant in &model.invariants {
            let applies = invariant
                .if_all
                .iter()
                .all(|condition| eval_condition(condition, &state, &predicates).unwrap_or(false));
            for condition in &invariant.if_all {
                validate_condition(condition, &state, &predicates, &model.id)?;
            }
            for condition in &invariant.then_all {
                validate_condition(condition, &state, &predicates, &model.id)?;
            }
            if applies
                && !invariant.then_all.iter().all(|condition| {
                    eval_condition(condition, &state, &predicates).unwrap_or(false)
                })
            {
                return Err(format!(
                    "{}: invariant `{}` failed at state ordinal {ordinal}",
                    model.id, invariant.id
                ));
            }
        }
    }
    Ok(count)
}

fn state_for_ordinal(
    dimensions: &[(String, Vec<String>)],
    mut ordinal: usize,
) -> BTreeMap<String, String> {
    let mut state = BTreeMap::new();
    for (name, values) in dimensions.iter().rev() {
        let index = ordinal % values.len();
        ordinal /= values.len();
        state.insert(name.clone(), values[index].clone());
    }
    state
}

fn validate_condition(
    condition: &str,
    state: &BTreeMap<String, String>,
    predicates: &BTreeMap<String, bool>,
    model: &str,
) -> CheckResult<()> {
    if eval_condition(condition, state, predicates).is_none() {
        return Err(format!(
            "{model}: unknown or malformed condition `{condition}`"
        ));
    }
    Ok(())
}

fn eval_condition(
    condition: &str,
    state: &BTreeMap<String, String>,
    predicates: &BTreeMap<String, bool>,
) -> Option<bool> {
    let (name, expected, unequal) = if let Some((name, value)) = condition.split_once("!=") {
        (name, value, true)
    } else {
        let (name, value) = condition.split_once('=')?;
        (name, value, false)
    };
    let actual = if let Some(value) = state.get(name) {
        value == expected
    } else if let Some(value) = predicates.get(name) {
        match expected {
            "true" => *value,
            "false" => !*value,
            _ => return None,
        }
    } else {
        return None;
    };
    Some(if unequal { !actual } else { actual })
}

fn validate_transition_model(model: &Model) -> CheckResult<usize> {
    if [
        model.states.is_empty(),
        model.initial.is_empty(),
        model.invariants.is_empty(),
    ]
    .contains(&true)
    {
        return Err(format!("{}: transition model is incomplete", model.id));
    }
    let mut states = BTreeMap::new();
    for state in &model.states {
        if [
            state.name.is_empty(),
            states.insert(state.name.clone(), state.terminal).is_some(),
        ]
        .contains(&true)
        {
            return Err(format!(
                "{}: invalid or duplicate state `{}`",
                model.id, state.name
            ));
        }
    }
    for initial in &model.initial {
        if !states.contains_key(initial) {
            return Err(format!("{}: unknown initial state `{initial}`", model.id));
        }
    }
    let mut transition_keys = BTreeSet::new();
    for transition in &model.transitions {
        if [
            !states.contains_key(&transition.from),
            !states.contains_key(&transition.to),
        ]
        .contains(&true)
        {
            return Err(format!("{}: transition references unknown state", model.id));
        }
        if [
            transition.event.is_empty(),
            !transition_keys.insert((transition.from.clone(), transition.event.clone())),
        ]
        .contains(&true)
        {
            return Err(format!("{}: duplicate/empty transition event", model.id));
        }
        if states.get(&transition.from) == Some(&true) {
            return Err(format!(
                "{}: terminal state has outgoing transition",
                model.id
            ));
        }
    }
    let mut reachable: BTreeSet<String> = model.initial.iter().cloned().collect();
    let mut queue: VecDeque<String> = model.initial.iter().cloned().collect();
    while let Some(state) = queue.pop_front() {
        let mut outgoing: Vec<_> = model
            .transitions
            .iter()
            .filter(|transition| transition.from == state)
            .collect();
        outgoing.sort_by(|left, right| {
            (&left.from, &left.event, &left.to).cmp(&(&right.from, &right.event, &right.to))
        });
        for transition in outgoing {
            if reachable.insert(transition.to.clone()) {
                queue.push_back(transition.to.clone());
            }
        }
    }
    if model.expected_reachable_count != Some(reachable.len()) {
        return Err(format!(
            "{}: expected_reachable_count {:?} does not equal {}",
            model.id,
            model.expected_reachable_count,
            reachable.len()
        ));
    }
    let mut invariant_ids = BTreeSet::new();
    for invariant in &model.invariants {
        if [
            invariant.id.is_empty(),
            !invariant_ids.insert(invariant.id.clone()),
        ]
        .contains(&true)
        {
            return Err(format!("{}: invalid invariant ID", model.id));
        }
        let relation = invariant
            .relation
            .as_deref()
            .ok_or_else(|| format!("{}: transition invariant lacks relation", model.id))?;
        match relation {
            "all-states-reachable" if reachable.len() == states.len() => {}
            "terminal-has-no-outgoing" => {}
            "terminal-never-reaches-live" => {}
            "id-remains-reserved" if model.states.iter().all(|state| state.id_reserved) => {}
            "id-remains-reserved" => {
                return Err(format!(
                    "{}: a lifecycle state releases its stable ID",
                    model.id
                ));
            }
            "all-states-reachable" => {
                return Err(format!("{}: not all states are reachable", model.id));
            }
            other => {
                return Err(format!(
                    "{}: unknown invariant relation `{other}`",
                    model.id
                ));
            }
        }
    }
    Ok(reachable.len())
}

fn collect_extension(root: &Path, extension: &str, output: &mut Vec<PathBuf>) -> CheckResult<()> {
    fs::read_dir(root)
        .map_err(|error| format!("read {}: {error}", root.display()))
        .and_then(|mut entries| collect_extension_entries(root, extension, output, &mut entries))
}

fn collect_extension_entries(
    root: &Path,
    extension: &str,
    output: &mut Vec<PathBuf>,
    entries: &mut dyn Iterator<Item = std::io::Result<fs::DirEntry>>,
) -> CheckResult<()> {
    while let Some(entry) = entries.next() {
        entry
            .map_err(|error| format!("read {} entry: {error}", root.display()))
            .and_then(|entry| {
                let path = entry.path();
                if path.is_dir() {
                    collect_extension(&path, extension, output)
                } else {
                    if path.extension().and_then(|value| value.to_str()) == Some(extension) {
                        output.push(path);
                    }
                    Ok(())
                }
            })?;
    }
    Ok(())
}

fn string_assignment(line: &str, key: &str) -> Option<String> {
    let prefix = format!("{key} = \"");
    let rest = line.strip_prefix(&prefix)?;
    let value = rest.strip_suffix('"')?;
    Some(value.to_owned())
}

fn usize_assignment(line: &str, key: &str) -> CheckResult<Option<usize>> {
    let prefix = format!("{key} = ");
    let Some(rest) = line.strip_prefix(&prefix) else {
        return Ok(None);
    };
    let value = rest
        .parse::<usize>()
        .map_err(|error| format!("invalid integer `{line}`: {error}"))?;
    Ok(Some(value))
}

fn bool_assignment(line: &str, key: &str) -> CheckResult<Option<bool>> {
    let prefix = format!("{key} = ");
    let Some(rest) = line.strip_prefix(&prefix) else {
        return Ok(None);
    };
    match rest {
        "true" => Ok(Some(true)),
        "false" => Ok(Some(false)),
        _ => Err(format!("invalid Boolean `{line}`")),
    }
}

fn array_assignment(line: &str, key: &str) -> CheckResult<Vec<String>> {
    let prefix = format!("{key} = ");
    let rest = line
        .strip_prefix(&prefix)
        .ok_or_else(|| format!("expected `{key}` assignment"))?;
    parse_quoted_array(rest)
}

fn inline_string(line: &str, key: &str) -> CheckResult<String> {
    let marker = format!("{key} = \"");
    let start = line
        .find(&marker)
        .ok_or_else(|| format!("missing inline `{key}`"))?
        + marker.len();
    let end = line[start..]
        .find('"')
        .ok_or_else(|| format!("unterminated inline `{key}`"))?
        + start;
    Ok(line[start..end].to_owned())
}

fn inline_array(line: &str, key: &str) -> CheckResult<Vec<String>> {
    let marker = format!("{key} = [");
    let start = line
        .find(&marker)
        .ok_or_else(|| format!("missing inline `{key}`"))?
        + marker.len()
        - 1;
    let end = line[start..]
        .find(']')
        .ok_or_else(|| format!("unterminated inline `{key}`"))?
        + start;
    parse_quoted_array(&line[start..=end])
}

fn parse_quoted_array(text: &str) -> CheckResult<Vec<String>> {
    let trimmed = text.trim();
    if [!trimmed.starts_with('['), !trimmed.ends_with(']')].contains(&true) {
        return Err(format!("invalid string array `{text}`"));
    }
    let mut values = Vec::new();
    let mut cursor = 1usize;
    let bytes = trimmed.as_bytes();
    while cursor + 1 < bytes.len() {
        while cursor + 1 < bytes.len()
            && (bytes[cursor].is_ascii_whitespace() || bytes[cursor] == b',')
        {
            cursor += 1;
        }
        if cursor + 1 >= bytes.len() {
            break;
        }
        if bytes[cursor] != b'"' {
            return Err(format!("array contains non-string value `{text}`"));
        }
        cursor += 1;
        let start = cursor;
        while cursor < bytes.len() && bytes[cursor] != b'"' {
            if bytes[cursor] == b'\\' {
                return Err(format!(
                    "escaped strings are not supported in fixture arrays `{text}`"
                ));
            }
            cursor += 1;
        }
        if cursor >= bytes.len() {
            return Err(format!("unterminated array string `{text}`"));
        }
        values.push(trimmed[start..cursor].to_owned());
        cursor += 1;
    }
    Ok(values)
}

fn find_top_string(text: &str, key: &str) -> Option<String> {
    text.lines()
        .map(str::trim)
        .take_while(|line| !line.starts_with('['))
        .find_map(|line| string_assignment(line, key))
}

fn find_top_array(text: &str, key: &str) -> CheckResult<Option<Vec<String>>> {
    for line in text
        .lines()
        .map(str::trim)
        .take_while(|line| !line.starts_with('['))
    {
        if line.starts_with(&format!("{key} = ")) {
            return array_assignment(line, key).map(Some);
        }
    }
    Ok(None)
}

fn slash_path(path: &Path) -> String {
    path.to_string_lossy().replace('\\', "/")
}

fn slugify(input: &str) -> String {
    let mut output = String::new();
    let mut separator = false;
    for character in input.chars().flat_map(char::to_lowercase) {
        if character.is_alphanumeric() {
            if separator && !output.is_empty() {
                output.push('-');
            }
            output.push(character);
            separator = false;
        } else {
            separator = true;
        }
    }
    output
}

fn scenario_match_key(slug: &str) -> String {
    const STOP_WORDS: [&str; 8] = ["a", "an", "the", "is", "are", "in", "to", "of"];
    slug.split('-')
        .filter(|word| !STOP_WORDS.contains(word))
        .collect::<Vec<_>>()
        .join("-")
}
