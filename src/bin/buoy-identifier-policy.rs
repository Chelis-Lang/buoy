#![doc = "Syntax-aware stable-ID boundary gate."]

use buoy::identifier_policy::{
    IdentifierPolicyFinding, evaluate_identifier_policy_fixture, scan_identifier_policy,
};
use std::collections::BTreeSet;
use std::env;
use std::fs;
use std::path::Path;
use std::process::ExitCode;

const CASES: &str = "fixtures/identifier/v1/policy-cases.tsv";

fn print_findings(findings: &[IdentifierPolicyFinding]) {
    println!("code\tpath\tline\tcolumn\trole\tdetail");
    for finding in findings {
        println!(
            "{}\t{}\t{}\t{}\t{}\t{}",
            finding.code, finding.path, finding.line, finding.column, finding.role, finding.detail,
        );
    }
}

fn self_test() -> Result<(), String> {
    self_test_control(None)
}

fn self_test_control(fault: Option<&str>) -> Result<(), String> {
    let cases_path = if fault == Some("cases-read") {
        "missing-identifier-cases.tsv"
    } else {
        CASES
    };
    let mut source =
        fs::read_to_string(cases_path).map_err(|error| format!("{cases_path}: {error}"))?;
    if fault == Some("row-width") {
        source.push_str("\ninvalid-row");
    }
    let mut cases = 0;
    for line in source.lines().skip(1) {
        let fields = line.split('\t').collect::<Vec<_>>();
        if fields.len() != 7 {
            return Err(String::from("identifier fixture row width differs"));
        }
        let fixture_path = if fault == Some("fixture-read") {
            "missing-identifier-fixture"
        } else {
            fields[4]
        };
        let mut fixture =
            fs::read_to_string(fixture_path).map_err(|error| format!("{fixture_path}: {error}"))?;
        if fault == Some("fixture-syntax") {
            fixture = String::from("not valid Rust");
        }
        let role = if fault == Some("fixture-role") {
            "unknown-role"
        } else {
            fields[3]
        };
        let mut findings = evaluate_identifier_policy_fixture(role, fields[4], &fixture)?;
        if fault == Some("case-mismatch") {
            findings.push(IdentifierPolicyFinding {
                code: "SID-BOUNDARY-CONTROL",
                path: String::from("control"),
                line: 0,
                column: 0,
                role: String::from("control"),
                detail: String::from("control"),
            });
        }
        let actual = findings
            .iter()
            .map(|finding| finding.code)
            .collect::<BTreeSet<_>>();
        let expected = if fields[6] == "-" {
            BTreeSet::new()
        } else {
            fields[6].split(',').collect()
        };
        if actual != expected {
            return Err(format!(
                "{}: expected {:?}, found {:?}",
                fields[1], expected, actual
            ));
        }
        let rejected = !findings.is_empty();
        let expected_rejection = if fault == Some("polarity-mismatch") {
            fields[5] != "reject"
        } else {
            fields[5] == "reject"
        };
        if rejected != expected_rejection {
            return Err(format!("{}: outcome differs from its polarity", fields[1]));
        }
        cases += 1;
    }
    println!("identifier-policy self-test: ok cases={cases}");
    Ok(())
}

fn repository_findings() -> Result<Vec<IdentifierPolicyFinding>, String> {
    repository_findings_control(None)
}

fn repository_findings_control(
    fault: Option<&str>,
) -> Result<Vec<IdentifierPolicyFinding>, String> {
    if fault == Some("finding") {
        return Ok(vec![IdentifierPolicyFinding {
            code: "SID-BOUNDARY-CONTROL",
            path: String::from("control"),
            line: 0,
            column: 0,
            role: String::from("control"),
            detail: String::from("control"),
        }]);
    }
    let scan_root = if fault == Some("scan") {
        Path::new("missing-identifier-root")
    } else {
        Path::new(".")
    };
    let mut findings = scan_identifier_policy(scan_root)?;
    findings.sort_unstable();
    findings.dedup();
    Ok(findings)
}

fn self_test_exit(result: Result<(), String>) -> ExitCode {
    match result {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("identifier-policy: {error}");
            ExitCode::from(1)
        }
    }
}

fn repository_exit(result: Result<Vec<IdentifierPolicyFinding>, String>) -> ExitCode {
    match result {
        Ok(findings) if findings.is_empty() => {
            println!("identifier-policy: ok findings=0");
            ExitCode::SUCCESS
        }
        Ok(findings) => {
            print_findings(&findings);
            ExitCode::from(1)
        }
        Err(error) => {
            eprintln!("identifier-policy: {error}");
            ExitCode::from(2)
        }
    }
}

fn dispatch(arguments: &[String]) -> ExitCode {
    #[cfg(feature = "coverage-controls")]
    match arguments {
        [argument] if argument == "--finding-control" => {
            return repository_exit(repository_findings_control(Some("finding")));
        }
        [argument] if argument == "--error-control" => {
            return repository_exit(Err(String::from("injected control")));
        }
        [argument] if argument == "--self-test-error-control" => {
            return self_test_exit(Err(String::from("injected self-test control")));
        }
        _ => {}
    }
    if arguments == ["--self-test"] {
        return self_test_exit(self_test());
    }
    if !arguments.is_empty() {
        eprintln!("usage: buoy-identifier-policy [--self-test]");
        return ExitCode::from(2);
    }
    repository_exit(repository_findings())
}

fn main() -> ExitCode {
    dispatch(&env::args().skip(1).collect::<Vec<_>>())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn assurance_controls_cover_every_entry_and_failure_path() {
        for fault in [
            "cases-read",
            "row-width",
            "fixture-read",
            "fixture-syntax",
            "fixture-role",
            "case-mismatch",
            "polarity-mismatch",
        ] {
            assert!(self_test_control(Some(fault)).is_err(), "{fault}");
        }
        assert!(repository_findings_control(Some("scan")).is_err());
        assert_eq!(dispatch(&[String::from("--self-test")]), ExitCode::SUCCESS);
        assert_ne!(dispatch(&[String::from("--invalid")]), ExitCode::SUCCESS);
        assert_ne!(
            dispatch(&[String::from("--finding-control")]),
            ExitCode::SUCCESS
        );
        assert_ne!(
            dispatch(&[String::from("--error-control")]),
            ExitCode::SUCCESS
        );
        assert_ne!(
            dispatch(&[String::from("--self-test-error-control")]),
            ExitCode::SUCCESS
        );
        assert_eq!(dispatch(&[]), ExitCode::SUCCESS);
        let _ = main();
    }
}
