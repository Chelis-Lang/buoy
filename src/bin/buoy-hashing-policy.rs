#![doc = "Syntax-aware repository hashing-policy gate."]

use buoy::hashing_policy::{
    HashingPolicyFinding, evaluate_hashing_policy_fixture, inspect_resolved_dependency_features,
    scan_hashing_policy,
};
use std::collections::BTreeSet;
use std::env;
use std::fs;
use std::path::Path;
use std::process::{Command, ExitCode};

const CASES: &str = "fixtures/xxh3/v2/policy-cases.tsv";

fn print_findings(findings: &[HashingPolicyFinding]) {
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
        "missing-policy-cases.tsv"
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
            return Err(String::from("policy fixture row width differs"));
        }
        let fixture_path = if fault == Some("fixture-read") {
            "missing-policy-fixture"
        } else {
            fields[4]
        };
        let mut fixture =
            fs::read_to_string(fixture_path).map_err(|error| format!("{fixture_path}: {error}"))?;
        if fault == Some("fixture-syntax") {
            fixture = String::from("not valid Rust");
        }
        let mut findings = evaluate_hashing_policy_fixture(fields[3], fields[4], &fixture)?;
        if fault == Some("case-mismatch") {
            findings.push(HashingPolicyFinding {
                code: "XXH-POLICY-CONTROL",
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
        cases += 1;
    }
    let positive_metadata = if fault == Some("positive-metadata-parse") {
        "not-json"
    } else if fault == Some("positive-metadata") {
        r#"{"packages":[{"name":"hidden","features":["sha2"]}]}"#
    } else {
        r#"{"packages":[{"name":"xxhash-rust","features":["xxh3"]}]}"#
    };
    if !inspect_resolved_dependency_features(positive_metadata)?.is_empty() {
        return Err(String::from("positive metadata unexpectedly failed"));
    }
    let negative_metadata = if fault == Some("negative-metadata-parse") {
        "not-json"
    } else if fault == Some("negative-metadata") {
        r#"{"packages":[]}"#
    } else {
        r#"{"packages":[{"name":"hidden","features":["sha2"]}]}"#
    };
    let mut negative = inspect_resolved_dependency_features(negative_metadata)?;
    if fault == Some("negative-metadata-code") {
        negative[0].code = "XXH-POLICY-CONTROL";
    }
    if negative.len() != 1 || negative[0].code != "XXH-POLICY-PROHIBITED-TRANSITIVE-FEATURE" {
        return Err(String::from(
            "metadata negative control did not fail exactly",
        ));
    }
    println!("hashing-policy self-test: ok cases={cases} metadata-controls=2");
    Ok(())
}

fn repository_findings() -> Result<Vec<HashingPolicyFinding>, String> {
    repository_findings_control(None)
}

fn repository_findings_control(fault: Option<&str>) -> Result<Vec<HashingPolicyFinding>, String> {
    if fault == Some("finding") {
        return Ok(vec![HashingPolicyFinding {
            code: "XXH-POLICY-CONTROL",
            path: String::from("control"),
            line: 0,
            column: 0,
            role: String::from("control"),
            detail: String::from("control"),
        }]);
    }
    let scan_root = if fault == Some("scan") {
        Path::new("missing-policy-root")
    } else {
        Path::new(".")
    };
    let mut findings = scan_hashing_policy(scan_root)?;
    let program = if fault == Some("metadata-spawn") {
        "missing-cargo-control"
    } else {
        "cargo"
    };
    let arguments = if fault == Some("metadata-status") {
        vec!["not-a-cargo-command"]
    } else {
        vec!["metadata", "--format-version", "1", "--locked", "--offline"]
    };
    let output = Command::new(program)
        .args(arguments)
        .output()
        .map_err(|error| format!("cargo metadata: {error}"))?;
    if !output.status.success() {
        return Err(String::from("cargo metadata failed"));
    }
    let stdout = if fault == Some("metadata-utf8") {
        vec![0xff]
    } else if fault == Some("metadata-parse") {
        b"not-json".to_vec()
    } else {
        output.stdout
    };
    let metadata =
        String::from_utf8(stdout).map_err(|_| String::from("cargo metadata emitted non-UTF-8"))?;
    findings.extend(inspect_resolved_dependency_features(&metadata)?);
    findings.sort_unstable();
    findings.dedup();
    Ok(findings)
}

fn self_test_exit(result: Result<(), String>) -> ExitCode {
    match result {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("hashing-policy: {error}");
            ExitCode::from(1)
        }
    }
}

fn repository_exit(result: Result<Vec<HashingPolicyFinding>, String>) -> ExitCode {
    match result {
        Ok(findings) if findings.is_empty() => {
            println!("hashing-policy: ok findings=0");
            ExitCode::SUCCESS
        }
        Ok(findings) => {
            print_findings(&findings);
            ExitCode::from(1)
        }
        Err(error) => {
            eprintln!("hashing-policy: {error}");
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
        eprintln!("usage: buoy-hashing-policy [--self-test]");
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
            "case-mismatch",
            "positive-metadata-parse",
            "positive-metadata",
            "negative-metadata-parse",
            "negative-metadata",
            "negative-metadata-code",
        ] {
            assert!(self_test_control(Some(fault)).is_err(), "{fault}");
        }
        for fault in [
            "scan",
            "metadata-spawn",
            "metadata-status",
            "metadata-utf8",
            "metadata-parse",
        ] {
            assert!(repository_findings_control(Some(fault)).is_err(), "{fault}");
        }
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
