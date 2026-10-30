//! Validate the digest-registry fixture contract and exact negative diagnostics.

#![allow(clippy::expect_used, clippy::too_many_lines, clippy::unwrap_used)]

#[path = "digest_binding.rs"]
mod content_digest;
#[path = "execution_evidence.rs"]
mod execution_evidence;

use std::collections::{BTreeMap, BTreeSet};
use std::env;
use std::fs;

const CASES: &str = "fixtures/digest/v3/cases.tsv";
const MANIFEST: &str = "fixtures/digest/v3/fixture-manifest.tsv";
const VECTORS: &str = "fixtures/digest/v3/known-answer-vectors.tsv";
const EXECUTIONS: &str = "assurance/v1/digest-fixture-executions.tsv";
const EXECUTION_TOOLS: [&str; 3] = [
    "tools/check_digest_fixtures.rs",
    "tools/execution_evidence.rs",
    "tools/digest_binding.rs",
];
const EXECUTION_CONFIGURATION: [&str; 3] = [MANIFEST, VECTORS, "crates/buoy-core/src/value.rs"];
const CASE_SCHEMA: &str = "buoy-digest-fixture/v3";
const MANIFEST_SCHEMA: &str = "buoy-digest-fixture-manifest/v3";
const VECTOR_SCHEMA: &str = "buoy-digest-known-answer/v3";
const CASE_HEADER: &str = "schema\trequirement\tcase\tpolarity\toperation\tinput\texpected_code";
const MANIFEST_HEADER: &str =
    "schema\trequirement\tscenario\tpolarity\tfixture_id\texpected\tfocused_command";
const VECTOR_HEADER: &str =
    "schema\talgorithm\tcase\tsource\trevision\tinput_encoding\tinput\toutput_hex";

#[derive(Clone, Debug)]
struct Case {
    requirement: String,
    id: String,
    polarity: String,
    operation: String,
    input: BTreeMap<String, String>,
    expected: String,
}

pub(crate) fn main() {
    if let Err(error) = run() {
        eprintln!("digest-fixture-check: {error}");
        std::process::exit(1);
    }
}

fn run() -> Result<(), String> {
    let arguments = env::args().skip(1).collect::<Vec<_>>();
    match arguments.as_slice() {
        [] => validate_repository(false),
        [argument] if argument == "--self-test" => self_test(),
        [argument] if argument == "--write-execution-evidence" => validate_repository(true),
        _ => Err(String::from(
            "usage: check-digest-fixtures [--self-test | --write-execution-evidence]",
        )),
    }
}

fn read_table(path: &str, expected_header: &str) -> Result<Vec<Vec<String>>, String> {
    let source = fs::read_to_string(path).map_err(|error| format!("read {path}: {error}"))?;
    let mut lines = source.lines();
    if lines.next() != Some(expected_header) {
        return Err(format!("{path}: header differs from the frozen schema"));
    }
    lines
        .enumerate()
        .map(|(index, line)| {
            if line.is_empty() {
                return Err(format!("{path}:{}: empty row", index + 2));
            }
            Ok(line.split('\t').map(str::to_owned).collect::<Vec<_>>())
        })
        .collect()
}

fn parse_input(value: &str, case: &str) -> Result<BTreeMap<String, String>, String> {
    let mut fields = BTreeMap::new();
    for item in value.split(';') {
        let (name, value) = item
            .split_once('=')
            .ok_or_else(|| format!("{case}: malformed input field `{item}`"))?;
        if name.is_empty()
            || value.is_empty()
            || fields.insert(name.to_owned(), value.to_owned()).is_some()
        {
            return Err(format!("{case}: duplicate or empty input field `{name}`"));
        }
    }
    Ok(fields)
}

fn parse_case_rows(rows: Vec<Vec<String>>) -> Result<Vec<Case>, String> {
    let mut ids = BTreeSet::new();
    rows.into_iter()
        .enumerate()
        .map(|(index, row)| {
            if row.len() != 7 {
                return Err(format!("{CASES}:{}: expected 7 columns", index + 2));
            }
            if row[0] != CASE_SCHEMA {
                return Err(format!("{}: wrong schema", row[2]));
            }
            if !ids.insert(row[2].clone()) {
                return Err(format!("{}: duplicate case", row[2]));
            }
            if !matches!(row[3].as_str(), "positive" | "negative") {
                return Err(format!("{}: invalid polarity", row[2]));
            }
            Ok(Case {
                requirement: row[1].clone(),
                id: row[2].clone(),
                polarity: row[3].clone(),
                operation: row[4].clone(),
                input: parse_input(&row[5], &row[2])?,
                expected: row[6].clone(),
            })
        })
        .collect()
}

fn parse_cases_from(path: &str) -> Result<Vec<Case>, String> {
    parse_case_rows(read_table(path, CASE_HEADER)?)
}

fn parse_cases() -> Result<Vec<Case>, String> {
    parse_cases_from(CASES)
}

fn field<'a>(case: &'a Case, name: &str) -> Result<&'a str, String> {
    case.input
        .get(name)
        .map(String::as_str)
        .ok_or_else(|| format!("{}: missing input field `{name}`", case.id))
}

#[allow(clippy::unnecessary_wraps)]
fn evaluate_registry(case: &Case) -> Result<&'static str, String> {
    if let Some(tag) = case.input.get("tag") {
        if matches!(tag.as_str(), "0" | "1") {
            return Ok("DIG-REGISTRY-OBSOLETE-TAG");
        }
    }
    if let (Some(registry), Some(inventory)) =
        (case.input.get("registry"), case.input.get("inventory"))
    {
        if registry != inventory && inventory != "same" {
            return Ok("DIG-REGISTRY-MISMATCH");
        }
    }
    Ok("pass")
}

fn algorithm_by_name(name: &str) -> Option<(u16, usize)> {
    match name {
        "xxh3-128" => Some((2, 16)),
        "blake3-256" => Some((3, 32)),
        "sha-256" => Some((4, 32)),
        _ => None,
    }
}

fn algorithm_by_tag(tag: u16) -> Option<(&'static str, usize)> {
    match tag {
        2 => Some(("xxh3-128", 16)),
        3 => Some(("blake3-256", 32)),
        4 => Some(("sha-256", 32)),
        _ => None,
    }
}

fn evaluate_parse(case: &Case) -> Result<&'static str, String> {
    let from_name = case.input.get("name").map(String::as_str);
    let from_tag = case
        .input
        .get("tag")
        .map(|tag| {
            tag.parse::<u16>()
                .map_err(|_| format!("{}: invalid tag", case.id))
        })
        .transpose()?;
    let expected_length = if let Some(name) = from_name {
        let Some((_, length)) = algorithm_by_name(name) else {
            return Ok("DET-DIGEST-ALGORITHM");
        };
        length
    } else if let Some(tag) = from_tag {
        let Some((_, length)) = algorithm_by_tag(tag) else {
            return Ok("DET-DIGEST-ALGORITHM");
        };
        length
    } else {
        return Err(format!("{}: parse case lacks a name or tag", case.id));
    };
    let hex = field(case, "hex")?;
    if !hex.bytes().all(|byte| byte.is_ascii_hexdigit())
        || hex.bytes().any(|byte| byte.is_ascii_uppercase())
    {
        return Ok("DET-DIGEST-HEX");
    }
    if let Some(length) = case.input.get("length") {
        if length.parse::<usize>().ok() != Some(expected_length) {
            return Ok("DET-DIGEST-LENGTH");
        }
    }
    if hex.len() != expected_length * 2 {
        return Ok("DET-DIGEST-LENGTH");
    }
    Ok("pass")
}

fn evaluate_active(case: &Case) -> Result<&'static str, String> {
    let declared = field(case, "declared")?;
    if declared == "-" {
        return Ok("DIG-ACTIVE-MISSING");
    }
    if declared.split(',').count() != 1 {
        return Ok("DIG-ACTIVE-MULTIPLE");
    }
    if field(case, "registered")? != "true" {
        return Ok("DIG-ALGORITHM-UNKNOWN");
    }
    if field(case, "feature")? != "true" {
        return Ok("DIG-ALGORITHM-UNAVAILABLE");
    }
    Ok("pass")
}

fn evaluate_boundary(case: &Case) -> Result<&'static str, String> {
    match field(case, "kind")? {
        "implementation" | "direct-call" => Ok("DIG-HASH-OUTSIDE-REGISTRY"),
        "adapter" if case.input.get("member") != case.input.get("active") => {
            Ok("DIG-HASH-WRONG-ADAPTER")
        }
        "compute" if case.input.get("member") != case.input.get("active") => {
            Ok("DIG-INACTIVE-ALGORITHM")
        }
        "adapter" | "compute" => Ok("pass"),
        other => Err(format!("{}: unknown boundary kind `{other}`", case.id)),
    }
}

fn evaluate_transition(case: &Case) -> Result<&'static str, String> {
    match field(case, "operation")? {
        "compare" | "order" => Ok("DIG-ALGORITHM-MISMATCH"),
        "relabel" | "substitute" => Ok("DIG-REGENERATION-REQUIRED"),
        "regenerate" if case.input.get("source").map(String::as_str) != Some("true") => {
            Ok("DIG-INCOMPLETE-MIGRATION")
        }
        "regenerate" if case.input.get("approval").map(String::as_str) != Some("true") => {
            Ok("DIG-INCOMPLETE-MIGRATION")
        }
        "regenerate" => Ok("pass"),
        other => Err(format!(
            "{}: unknown transition operation `{other}`",
            case.id
        )),
    }
}

fn evaluate_claim(case: &Case) -> Result<&'static str, String> {
    let active = field(case, "active")?;
    let claim = field(case, "claim")?;
    if matches!(claim, "authenticity" | "tamper-evidence" | "proof") {
        return Ok("DIG-UNSUPPORTED-ASSURANCE-CLAIM");
    }
    if claim == "collision-resistant" && active == "xxh3-128" {
        return Ok("DIG-UNSUPPORTED-ASSURANCE-CLAIM");
    }
    Ok("pass")
}

fn evaluate_index(case: &Case) -> Result<&'static str, String> {
    let key = field(case, "key")?;
    if algorithm_by_name(key.split(':').next().unwrap_or("")).is_some() && key.contains(':') {
        return Ok("DIG-INDEX-DIGEST-KEY");
    }
    if case.input.get("graph").map(String::as_str) == Some("false") {
        return Ok("DIG-INDEX-OBJECT-MISSING");
    }
    if case.input.get("container").map(String::as_str) == Some("binary") {
        return Ok("DIG-INDEX-NONREPRODUCIBLE");
    }
    if case.input.get("old").map(String::as_str) == Some("contradicted")
        || case.input.get("new").map(String::as_str) == Some("contradicted")
    {
        return Ok("DIG-INDEX-CORRESPONDENCE");
    }
    Ok("pass")
}

#[allow(clippy::unnecessary_wraps)]
fn evaluate_identity(case: &Case) -> Result<&'static str, String> {
    if case.input.get("source").map(String::as_str) == Some("false") {
        return Ok("DIG-INCOMPLETE-MIGRATION");
    }
    if case.input.get("freshness").map(String::as_str) == Some("index")
        || case.input.get("identity").map(String::as_str) == Some("index")
    {
        return Ok("DIG-INDEX-AUTHORITY");
    }
    Ok("pass")
}

fn evaluate_external(case: &Case) -> Result<&'static str, String> {
    if field(case, "use")? == "semantic" {
        Ok("XXH-EXTERNAL-SEMANTIC-PROMOTION")
    } else {
        Ok("pass")
    }
}

fn evaluate(case: &Case) -> Result<&'static str, String> {
    match case.operation.as_str() {
        "registry" => evaluate_registry(case),
        "parse" => evaluate_parse(case),
        "active" => evaluate_active(case),
        "boundary" => evaluate_boundary(case),
        "transition" => evaluate_transition(case),
        "claim" => evaluate_claim(case),
        "index" => evaluate_index(case),
        "identity" => evaluate_identity(case),
        "external" => evaluate_external(case),
        "collision" => Ok("DET-DIGEST-OBSERVED-COLLISION"),
        other => Err(format!("{}: unknown operation `{other}`", case.id)),
    }
}

fn validate_cases(cases: &[Case]) -> Result<(), String> {
    for case in cases {
        if !(case.requirement.starts_with("DIG-") || case.requirement.starts_with("XXH-")) {
            return Err(format!("{}: unknown requirement family", case.id));
        }
        let actual = evaluate(case)?;
        if actual != case.expected {
            return Err(format!(
                "{}: expected diagnostic `{}`, got `{actual}`",
                case.id, case.expected
            ));
        }
        if (case.polarity == "positive") != (actual == "pass") {
            return Err(format!("{}: polarity does not match `{actual}`", case.id));
        }
    }
    Ok(())
}

fn validate_vector_rows(rows: &[Vec<String>]) -> Result<(), String> {
    let mut algorithms = BTreeSet::new();
    let mut cases = BTreeSet::new();
    for (index, row) in rows.iter().enumerate() {
        if row.len() != 8 || row[0] != VECTOR_SCHEMA {
            return Err(format!("{VECTORS}:{}: invalid row", index + 2));
        }
        let Some((_, length)) = algorithm_by_name(&row[1]) else {
            return Err(format!("{VECTORS}:{}: unknown algorithm", index + 2));
        };
        if row[3].is_empty() || row[4].is_empty() {
            return Err(format!(
                "{VECTORS}:{}: source or revision is empty",
                index + 2
            ));
        }
        if row[7].len() != length * 2
            || !row[7]
                .bytes()
                .all(|byte| byte.is_ascii_hexdigit() && !byte.is_ascii_uppercase())
        {
            return Err(format!("{VECTORS}:{}: invalid output", index + 2));
        }
        if !cases.insert(format!("{}:{}", row[1], row[2])) {
            return Err(format!("{VECTORS}:{}: duplicate vector", index + 2));
        }
        algorithms.insert(row[1].clone());
    }
    let expected = ["blake3-256", "sha-256", "xxh3-128"]
        .into_iter()
        .map(str::to_owned)
        .collect::<BTreeSet<_>>();
    if algorithms != expected {
        return Err(String::from(
            "known-answer vectors do not cover every member",
        ));
    }
    Ok(())
}

fn validate_vectors_from(path: &str) -> Result<(), String> {
    validate_vector_rows(&read_table(path, VECTOR_HEADER)?)
}

fn validate_external_fixture(fixture: &str, row: &[String]) -> Result<bool, String> {
    let expected = match fixture {
        "known-answer-vectors" => Some((
            "positive",
            "pass",
            "cargo run --quiet --bin check-digest-fixtures",
        )),
        "model-full-space" | "model-index-projection" => Some((
            "positive",
            "pass",
            "cargo run --quiet --bin check-digest-transition-model",
        )),
        "model-negative-controls" => Some((
            "negative",
            "DIG-MODEL-INDEX-AUTHORITY",
            "cargo run --quiet --bin check-digest-transition-model -- --fault model-index-authority",
        )),
        "model-undeclared-cutoff" => Some((
            "negative",
            "DIG-MODEL-CUTOFF",
            "cargo run --quiet --bin check-digest-transition-model -- --cutoff 4095",
        )),
        _ => None,
    };
    let Some((polarity, diagnostic, command)) = expected else {
        return Ok(false);
    };
    if row[3] != polarity || row[5] != diagnostic || row[6] != command {
        return Err(format!(
            "{MANIFEST}: external fixture `{fixture}` contract differs"
        ));
    }
    Ok(true)
}

fn validate_manifest_rows(cases: &[Case], rows: &[Vec<String>]) -> Result<(), String> {
    let case_map = cases
        .iter()
        .map(|case| (case.id.as_str(), case))
        .collect::<BTreeMap<_, _>>();
    let mut requirement_polarities = BTreeMap::<String, BTreeSet<String>>::new();
    let mut scenarios = BTreeSet::new();
    for (index, row) in rows.iter().enumerate() {
        if row.len() != 7 || row[0] != MANIFEST_SCHEMA {
            return Err(format!("{MANIFEST}:{}: invalid row", index + 2));
        }
        if !scenarios.insert(format!("{}:{}", row[1], row[2])) {
            return Err(format!("{MANIFEST}:{}: duplicate scenario", index + 2));
        }
        requirement_polarities
            .entry(row[1].clone())
            .or_default()
            .insert(row[3].clone());
        for fixture in row[4].split(',') {
            if validate_external_fixture(fixture, row)? {
                continue;
            }
            let Some(case) = case_map.get(fixture) else {
                return Err(format!(
                    "{MANIFEST}:{}: unknown fixture `{fixture}`",
                    index + 2
                ));
            };
            if case.polarity != row[3] {
                return Err(format!(
                    "{MANIFEST}:{}: fixture polarity differs",
                    index + 2
                ));
            }
            if row[5] != "pass" && case.expected != row[5] {
                return Err(format!(
                    "{MANIFEST}:{}: fixture diagnostic differs",
                    index + 2
                ));
            }
        }
        if row[6].is_empty() {
            return Err(format!(
                "{MANIFEST}:{}: focused command is empty",
                index + 2
            ));
        }
    }
    let expected_requirements = (1..=9)
        .map(|number| format!("DIG-{number:03}"))
        .chain((1..=6).map(|number| format!("XXH-{number:03}")))
        .collect::<BTreeSet<_>>();
    if requirement_polarities
        .keys()
        .cloned()
        .collect::<BTreeSet<_>>()
        != expected_requirements
    {
        return Err(String::from(
            "fixture manifest requirement coverage differs",
        ));
    }
    let both = [String::from("negative"), String::from("positive")]
        .into_iter()
        .collect::<BTreeSet<_>>();
    if requirement_polarities
        .values()
        .any(|polarities| *polarities != both)
    {
        return Err(String::from(
            "each requirement needs both fixture polarities",
        ));
    }
    Ok(())
}

fn validate_manifest_from(cases: &[Case], path: &str) -> Result<(), String> {
    validate_manifest_rows(cases, &read_table(path, MANIFEST_HEADER)?)
}

fn validate_repository_from(
    cases_path: &str,
    vectors_path: &str,
    manifest_path: &str,
) -> Result<Vec<Case>, String> {
    let cases = parse_cases_from(cases_path)?;
    validate_cases(&cases)?;
    validate_vectors_from(vectors_path)?;
    validate_manifest_from(&cases, manifest_path)?;
    Ok(cases)
}

fn execution_records(
    cases: &[Case],
    rows: &[Vec<String>],
) -> Result<Vec<execution_evidence::Record>, String> {
    cases
        .iter()
        .zip(rows)
        .map(|(case, row)| {
            evaluate(case).and_then(|actual| {
                let result = format!(
                    "operation={};observed={actual};expected={}",
                    case.operation, case.expected
                );
                execution_evidence::passing(
                    "fixture-operation",
                    format!("{CASES}#{}#{}", case.id, case.operation),
                    CASES,
                    row.join("\t").as_bytes(),
                    &EXECUTION_TOOLS,
                    &EXECUTION_CONFIGURATION,
                    result,
                )
            })
        })
        .collect()
}

fn validate_repository(write_execution_evidence: bool) -> Result<(), String> {
    validate_repository_from(CASES, VECTORS, MANIFEST).and_then(|cases| {
        read_table(CASES, CASE_HEADER).and_then(|rows| {
            execution_records(&cases, &rows).and_then(|records| {
                execution_evidence::render(&records).and_then(|executions| {
                    execution_evidence::validate_materialized(
                        EXECUTIONS,
                        &executions,
                        write_execution_evidence,
                    )
                    .map(|()| {
                        let positive = cases
                            .iter()
                            .filter(|case| case.polarity == "positive")
                            .count();
                        println!(
                            "digest-fixture-check: ok cases={} positive={} negative={} requirements=15 executions={}",
                            cases.len(),
                            positive,
                            cases.len() - positive,
                            cases.len()
                        );
                    })
                })
            })
        })
    })
}

#[allow(clippy::unnecessary_wraps)]
fn self_test() -> Result<(), String> {
    validate_repository(false).expect("repository fixtures are valid");
    let controls = [
        ("registry", "tag=0;member=test", "DIG-REGISTRY-OBSOLETE-TAG"),
        (
            "registry",
            "registry=three;inventory=two",
            "DIG-REGISTRY-MISMATCH",
        ),
        ("parse", "name=xxh3-128;hex=00", "DET-DIGEST-LENGTH"),
        (
            "active",
            "declared=sha-256;registered=true;feature=false",
            "DIG-ALGORITHM-UNAVAILABLE",
        ),
        (
            "boundary",
            "path=outside;kind=direct-call",
            "DIG-HASH-OUTSIDE-REGISTRY",
        ),
        (
            "identity",
            "source=false;index=true;identity=index",
            "DIG-INCOMPLETE-MIGRATION",
        ),
    ];
    for (index, (operation, input, expected)) in controls.into_iter().enumerate() {
        let case = Case {
            requirement: String::from("CONTROL"),
            id: format!("control-{index}"),
            polarity: String::from("negative"),
            operation: operation.to_owned(),
            input: parse_input(input, "control").expect("control input is valid"),
            expected: expected.to_owned(),
        };
        let actual = evaluate(&case).expect("control evaluation succeeds");
        assert_eq!(actual, expected, "negative control {index} differs");
    }

    let external_row = [
        "schema",
        "DIG-001",
        "control",
        "positive",
        "known-answer-vectors",
        "pass",
        "cargo run --quiet --bin check-digest-fixtures",
    ]
    .map(String::from)
    .to_vec();
    assert!(
        validate_external_fixture("known-answer-vectors", &external_row)
            .expect("the matching external contract is accepted")
    );
    for (index, value) in [
        (3, "negative"),
        (5, "DIG-CONTROL"),
        (6, "cargo run --quiet"),
    ] {
        let mut mismatched = external_row.clone();
        mismatched[index] = String::from(value);
        assert!(
            validate_external_fixture("known-answer-vectors", &mismatched)
                .expect_err("a mismatched external contract must fail")
                .contains("contract differs")
        );
    }
    let mismatched_manifest_row = {
        let mut row = external_row.clone();
        row[0] = String::from(MANIFEST_SCHEMA);
        row[3] = String::from("negative");
        row
    };
    assert!(
        validate_manifest_rows(&[], &[mismatched_manifest_row])
            .expect_err("a mismatched external manifest row must fail")
            .contains("contract differs")
    );

    let synthetic = |operation: &str, input: &str| -> Case {
        Case {
            requirement: String::from("CONTROL"),
            id: format!("control-{operation}-{input}"),
            polarity: String::from("negative"),
            operation: operation.to_owned(),
            input: parse_input(input, "synthetic-control").expect("synthetic input is valid"),
            expected: String::from("control"),
        }
    };
    for input in [
        "tag=2;member=test",
        "registry=same;inventory=same",
        "registry=different;inventory=same",
    ] {
        assert_eq!(
            evaluate(&synthetic("registry", input)).expect("registry control succeeds"),
            "pass"
        );
    }
    let _ = evaluate(&synthetic("parse", "hex=00")).expect_err("missing selector must fail");
    let _ = evaluate(&synthetic("parse", "tag=invalid;hex=00")).expect_err("invalid tag must fail");
    let _ = evaluate(&synthetic("external", "value=present"))
        .expect_err("missing external use must fail");
    for (operation, input) in [
        ("parse", "name=xxh3-128"),
        ("active", "value=present"),
        ("active", "declared=xxh3-128;value=present"),
        ("active", "declared=xxh3-128;registered=true;value=present"),
        ("boundary", "value=present"),
        ("transition", "value=present"),
        ("claim", "claim=change-detection"),
        ("claim", "active=xxh3-128"),
        ("index", "value=present"),
    ] {
        let _ = evaluate(&synthetic(operation, input)).expect_err("missing field must fail");
    }
    let _ = evaluate(&synthetic("boundary", "kind=unknown"))
        .expect_err("unknown boundary kind must fail");
    assert_eq!(
        evaluate(&synthetic(
            "boundary",
            "kind=compute;member=same;active=same"
        ))
        .expect("boundary control succeeds"),
        "pass"
    );
    assert_eq!(
        evaluate(&synthetic(
            "claim",
            "active=sha-256;claim=collision-resistant"
        ))
        .expect("claim control succeeds"),
        "pass"
    );
    assert_eq!(
        evaluate(&synthetic(
            "claim",
            "active=xxh3-128;claim=change-detection"
        ))
        .expect("claim control succeeds"),
        "pass"
    );
    let _ = evaluate(&synthetic("index", "key=xxh3-128;old=contradicted"))
        .expect("index control succeeds");
    let _ = evaluate(&synthetic("identity", "source=true;freshness=index"))
        .expect("identity control succeeds");
    let _ = evaluate(&synthetic(
        "identity",
        "source=true;freshness=graph;identity=index",
    ))
    .expect("identity control succeeds");
    let _ = evaluate(&synthetic("transition", "operation=unknown"))
        .expect_err("unknown transition must fail");
    let _ =
        evaluate(&synthetic("unknown", "value=present")).expect_err("unknown operation must fail");
    for input in [
        "tag=2;hex=00000000000000000000000000000000",
        "tag=4;hex=0000000000000000000000000000000000000000000000000000000000000000",
    ] {
        let _ = evaluate(&synthetic("parse", input)).expect("parse control succeeds");
    }
    for input in [
        "operation=regenerate;source=false;approval=true",
        "operation=regenerate;source=true;approval=false",
    ] {
        let _ = evaluate(&synthetic("transition", input)).expect("transition control succeeds");
    }
    let _ = parse_input("malformed", "control").expect_err("malformed input must fail");
    let _ = parse_input("a=1;a=2", "control").expect_err("duplicate input must fail");
    let _ = parse_input("=empty", "control").expect_err("empty name must fail");
    let _ = parse_input("empty=", "control").expect_err("empty value must fail");

    let scratch = env::temp_dir().join(format!(
        "buoy-digest-fixture-controls-{}",
        std::process::id()
    ));
    fs::create_dir_all(&scratch).expect("create digest fixture control directory");
    let table = scratch.join("table.tsv");
    fs::write(&table, "wrong\n").expect("write digest fixture control table");
    let table_text = table.to_string_lossy();
    let _ = read_table(&table_text, "header").expect_err("wrong header must fail");
    fs::write(&table, "header\n\nrow").expect("rewrite digest fixture control table");
    let _ = read_table(&table_text, "header").expect_err("empty row must fail");
    let missing_table = scratch.join("missing.tsv");
    let missing_table_text = missing_table.to_string_lossy();
    let _ = read_table(&missing_table_text, "header").expect_err("missing table must fail");
    let _ = validate_repository_from(&missing_table_text, VECTORS, MANIFEST)
        .expect_err("missing case table must fail");
    let _ = validate_repository_from(CASES, &missing_table_text, MANIFEST)
        .expect_err("missing vector table must fail");
    let _ = validate_repository_from(CASES, VECTORS, &missing_table_text)
        .expect_err("missing manifest table must fail");

    let case_rows = read_table(CASES, CASE_HEADER).expect("case table is valid");
    let valid_case_row = case_rows[0].clone();
    let mut malformed = valid_case_row.clone();
    malformed.pop();
    let _ = parse_case_rows(vec![malformed]).expect_err("short case row must fail");
    let mut wrong_schema = valid_case_row.clone();
    wrong_schema[0] = String::from("wrong");
    let _ = parse_case_rows(vec![wrong_schema]).expect_err("case schema must fail");
    let _ = parse_case_rows(vec![valid_case_row.clone(), valid_case_row.clone()])
        .expect_err("duplicate case must fail");
    let mut wrong_polarity = valid_case_row.clone();
    wrong_polarity[3] = String::from("unknown");
    let _ = parse_case_rows(vec![wrong_polarity]).expect_err("case polarity must fail");
    let mut malformed_input = valid_case_row.clone();
    malformed_input[5] = String::from("malformed");
    let _ = parse_case_rows(vec![malformed_input]).expect_err("case input must fail");
    let mut invalid_requirement_row = valid_case_row;
    invalid_requirement_row[1] = String::from("UNKNOWN-001");
    let invalid_cases = scratch.join("invalid-cases.tsv");
    fs::write(
        &invalid_cases,
        format!("{CASE_HEADER}\n{}\n", invalid_requirement_row.join("\t")),
    )
    .expect("write invalid case control table");
    let _ = validate_repository_from(&invalid_cases.to_string_lossy(), VECTORS, MANIFEST)
        .expect_err("invalid repository cases must fail");

    let vector_rows = read_table(VECTORS, VECTOR_HEADER).expect("vector table is valid");
    let valid_vector = vector_rows[0].clone();
    let mut invalid_vector = valid_vector.clone();
    invalid_vector.pop();
    let _ = validate_vector_rows(&[invalid_vector]).expect_err("short vector must fail");
    let mut invalid_vector = valid_vector.clone();
    invalid_vector[0] = String::from("wrong");
    let _ = validate_vector_rows(&[invalid_vector]).expect_err("vector schema must fail");
    let mut invalid_vector = valid_vector.clone();
    invalid_vector[1] = String::from("unknown");
    let _ = validate_vector_rows(&[invalid_vector]).expect_err("vector member must fail");
    for column in [3_usize, 4] {
        let mut invalid_vector = valid_vector.clone();
        invalid_vector[column].clear();
        let _ = validate_vector_rows(&[invalid_vector]).expect_err("vector authority must fail");
    }
    for output in [
        "00",
        "AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA",
        "gggggggggggggggggggggggggggggggg",
    ] {
        let mut invalid_vector = valid_vector.clone();
        output.clone_into(&mut invalid_vector[7]);
        let _ = validate_vector_rows(&[invalid_vector]).expect_err("vector output must fail");
    }
    let _ = validate_vector_rows(&[valid_vector.clone(), valid_vector])
        .expect_err("duplicate vector must fail");
    let incomplete_vectors = vector_rows
        .iter()
        .filter(|row| row[1] != "sha-256")
        .cloned()
        .collect::<Vec<_>>();
    let _ = validate_vector_rows(&incomplete_vectors).expect_err("member coverage must fail");

    let cases = parse_cases().expect("case fixtures are valid");
    let manifest_rows = read_table(MANIFEST, MANIFEST_HEADER).expect("manifest table is valid");
    let valid_manifest = manifest_rows
        .iter()
        .find(|row| {
            row[4]
                .split(',')
                .any(|fixture| cases.iter().any(|case| case.id == fixture))
        })
        .cloned()
        .expect("manifest control fixture is present");
    let mut invalid_manifest = valid_manifest.clone();
    invalid_manifest.pop();
    let _ = validate_manifest_rows(&cases, &[invalid_manifest])
        .expect_err("short manifest row must fail");
    let mut invalid_manifest = valid_manifest.clone();
    invalid_manifest[0] = String::from("wrong");
    let _ =
        validate_manifest_rows(&cases, &[invalid_manifest]).expect_err("manifest schema must fail");
    let _ = validate_manifest_rows(&cases, &[valid_manifest.clone(), valid_manifest.clone()])
        .expect_err("duplicate scenario must fail");
    let mut invalid_manifest = valid_manifest.clone();
    invalid_manifest[4] = String::from("unknown-fixture");
    let _ =
        validate_manifest_rows(&cases, &[invalid_manifest]).expect_err("unknown fixture must fail");
    for polarity in ["positive", "negative"] {
        let mut invalid_manifest = manifest_rows
            .iter()
            .find(|row| {
                row[3] == polarity
                    && row[4]
                        .split(',')
                        .next()
                        .is_some_and(|fixture| cases.iter().any(|case| case.id == fixture))
            })
            .cloned()
            .expect("manifest polarity control fixture is present");
        invalid_manifest[3] = if polarity == "positive" {
            String::from("negative")
        } else {
            String::from("positive")
        };
        let _ = validate_manifest_rows(&cases, &[invalid_manifest])
            .expect_err("fixture polarity must fail");
    }
    let mut invalid_manifest = valid_manifest.clone();
    invalid_manifest[5] = String::from("wrong-diagnostic");
    let _ = validate_manifest_rows(&cases, &[invalid_manifest])
        .expect_err("fixture diagnostic must fail");
    let mut invalid_manifest = valid_manifest;
    invalid_manifest[6].clear();
    let _ = validate_manifest_rows(&cases, &[invalid_manifest])
        .expect_err("empty focused command must fail");
    let incomplete_requirements = manifest_rows
        .iter()
        .filter(|row| row[1] != "DIG-009")
        .cloned()
        .collect::<Vec<_>>();
    let _ = validate_manifest_rows(&cases, &incomplete_requirements)
        .expect_err("requirement coverage must fail");
    let incomplete_polarity = manifest_rows
        .iter()
        .filter(|row| !(row[1] == "DIG-009" && row[3] == "negative"))
        .cloned()
        .collect::<Vec<_>>();
    let _ = validate_manifest_rows(&cases, &incomplete_polarity)
        .expect_err("requirement polarity must fail");

    let mut invalid_evaluation = synthetic("parse", "hex=00");
    invalid_evaluation.requirement = String::from("DIG-001");
    let _ = validate_cases(&[invalid_evaluation]).expect_err("case evaluation must fail");
    let mut invalid_requirement = cases[0].clone();
    invalid_requirement.requirement = String::from("UNKNOWN-001");
    let _ = validate_cases(&[invalid_requirement]).expect_err("requirement family must fail");
    let mut invalid_polarity = cases[0].clone();
    invalid_polarity.polarity = String::from("negative");
    let _ = validate_cases(&[invalid_polarity]).expect_err("case polarity must fail");
    let mut mismatched = cases
        .into_iter()
        .find(|case| case.polarity == "negative")
        .expect("negative fixture set is not empty");
    mismatched.expected = String::from("WRONG-DIAGNOSTIC");
    let _ = validate_cases(&[mismatched]).expect_err("wrong diagnostic must fail");
    fs::remove_dir_all(&scratch).expect("remove digest fixture control directory");
    println!("digest-fixture-check: self-test ok controls=36");
    Ok(())
}
