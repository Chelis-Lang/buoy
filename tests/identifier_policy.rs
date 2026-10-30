#![doc = "Deterministic controls for the stable-ID boundary gate."]
#![allow(clippy::expect_used)]

use buoy::identifier_policy::{
    IdentifierPolicyFinding, evaluate_identifier_policy_fixture, scan_identifier_policy,
};
use std::fs;
use std::path::Path;

const CASES: &str = "fixtures/identifier/v1/policy-cases.tsv";

fn fixture(path: &str) -> String {
    fs::read_to_string(path).expect("identifier policy fixture must read")
}

fn evaluate(role: &str, path: &str) -> Vec<IdentifierPolicyFinding> {
    evaluate_identifier_policy_fixture(role, path, &fixture(path))
        .expect("identifier policy fixture must parse")
}

fn rows(findings: &[IdentifierPolicyFinding]) -> Vec<(&str, String, usize, usize, String, String)> {
    findings
        .iter()
        .map(|finding| {
            (
                finding.code,
                finding.path.clone(),
                finding.line,
                finding.column,
                finding.role.clone(),
                finding.detail.clone(),
            )
        })
        .collect()
}

/// Task 6.1 - the conforming fixtures produce an empty finding set.
#[test]
fn conforming_identifier_fixtures_yield_no_findings() {
    let model = "fixtures/identifier/v1/policy/positive/model.rs.txt";
    let identifier = "fixtures/identifier/v1/policy/positive/identifier.rs.txt";
    assert_eq!(evaluate("core-model", model), Vec::new());
    assert_eq!(evaluate("identifier-module", identifier), Vec::new());
}

/// Task 6.2 - an untyped stable-ID parameter reports exact ordered findings.
#[test]
fn untyped_stable_id_parameter_reports_exact_findings() {
    let path = "fixtures/identifier/v1/policy/negative/untyped-parameter.rs.txt";
    assert_eq!(
        rows(&evaluate("core-model", path)),
        vec![
            (
                "SID-BOUNDARY-UNTYPED-PARAMETER",
                path.to_owned(),
                4,
                16,
                String::from("core-model"),
                String::from("Atom::new(id)"),
            ),
            (
                "SID-BOUNDARY-UNTYPED-PARAMETER",
                path.to_owned(),
                12,
                30,
                String::from("core-model"),
                String::from("ChangeDisposition::new(impacted_id)"),
            ),
        ]
    );
}

/// Task 6.2 - a stable-ID string bypass reports exact ordered findings.
#[test]
fn stable_id_string_bypass_reports_exact_findings() {
    let path = "fixtures/identifier/v1/policy/negative/string-bypass.rs.txt";
    assert_eq!(
        rows(&evaluate("core-model", path)),
        vec![
            (
                "SID-BOUNDARY-STRING-BYPASS",
                path.to_owned(),
                8,
                19,
                String::from("core-model"),
                String::from("ObjectRef::from_validated"),
            ),
            (
                "SID-BOUNDARY-UNTYPED-PARAMETER",
                path.to_owned(),
                8,
                52,
                String::from("core-model"),
                String::from("ObjectRef::from_validated(id)"),
            ),
        ]
    );
}

/// Task 6.1 - scope, schema, and prefix boundaries report exact ordered findings.
#[test]
fn remaining_identifier_boundaries_report_exact_findings() {
    let path = "fixtures/identifier/v1/policy/negative/scope-schema-prefix-boundaries.rs.txt";
    assert_eq!(
        rows(&evaluate("core-model", path)),
        vec![
            (
                "SID-BOUNDARY-EXTERNAL-GRAMMAR",
                path.to_owned(),
                6,
                9,
                String::from("core-model"),
                String::from("require_scope_id")
            ),
            (
                "SID-BOUNDARY-EXTERNAL-GRAMMAR",
                path.to_owned(),
                7,
                9,
                String::from("core-model"),
                String::from("require_schema_id")
            ),
            (
                "SID-BOUNDARY-UNTYPED-PARAMETER",
                path.to_owned(),
                3,
                14,
                String::from("core-model"),
                String::from("Boundary::scope(scope_id)")
            ),
            (
                "SID-BOUNDARY-UNTYPED-PARAMETER",
                path.to_owned(),
                4,
                15,
                String::from("core-model"),
                String::from("Boundary::schema(schema_id)")
            ),
        ]
    );
}

/// Task 6.2 - a grammar check outside the identifier module reports exact findings.
#[test]
fn external_stable_id_grammar_reports_exact_findings() {
    let path = "fixtures/identifier/v1/policy/negative/external-grammar.rs.txt";
    assert_eq!(
        rows(&evaluate("core-model", path)),
        vec![
            (
                "SID-BOUNDARY-EXTERNAL-GRAMMAR",
                path.to_owned(),
                3,
                4,
                String::from("core-model"),
                String::from("require_stable_id"),
            ),
            (
                "SID-BOUNDARY-EXTERNAL-GRAMMAR",
                path.to_owned(),
                4,
                8,
                String::from("core-model"),
                String::from("stable_id_grammar"),
            ),
            (
                "SID-BOUNDARY-EXTERNAL-GRAMMAR",
                path.to_owned(),
                13,
                9,
                String::from("core-model"),
                String::from("require_stable_id"),
            ),
        ]
    );
}

/// Task 6.2 - every declared case agrees with its fixture and polarity.
#[test]
fn every_declared_identifier_case_matches_its_fixture() {
    let source = fs::read_to_string(CASES).expect("identifier policy cases must read");
    let mut cases = 0;
    for line in source.lines().skip(1) {
        let fields = line.split('\t').collect::<Vec<_>>();
        assert_eq!(fields.len(), 7, "row width differs");
        assert_eq!(fields[0], "buoy-identifier-policy-case/v1");
        let findings = evaluate(fields[3], fields[4]);
        let actual = findings
            .iter()
            .map(|finding| finding.code)
            .collect::<std::collections::BTreeSet<_>>();
        let expected = if fields[6] == "-" {
            std::collections::BTreeSet::new()
        } else {
            fields[6].split(',').collect()
        };
        assert_eq!(actual, expected, "{}", fields[1]);
        assert_eq!(
            findings.is_empty(),
            fields[5] == "accept",
            "{} outcome differs from its polarity",
            fields[1]
        );
        cases += 1;
    }
    assert_eq!(cases, 6);
}

/// Task 6.1 - the converted repository has no stable-ID boundary regression.
#[test]
fn converted_repository_has_no_identifier_boundary_findings() {
    let findings = scan_identifier_policy(Path::new(".")).expect("repository scan must succeed");
    assert_eq!(rows(&findings), Vec::new());
}

/// An unknown fixture role fails closed.
#[test]
fn unknown_identifier_role_fails_closed() {
    assert!(evaluate_identifier_policy_fixture("unknown", "control.rs", "fn main() {}").is_err());
}

/// Malformed fixture syntax fails closed.
#[test]
fn malformed_identifier_fixture_fails_closed() {
    assert!(
        evaluate_identifier_policy_fixture("core-model", "control.rs", "not valid Rust").is_err()
    );
    let structural = r"
        mod nested {
            trait Boundary {
                const NAME: &'static str;
                fn inspect(id: &'static str);
                fn tuple(tuple_id: (&'static str, String));
                fn ignore(_: (&'static str, String));
            }
            struct SchemaImpactKey;
            impl SchemaImpactKey {
                fn new(other_id: &'static str) {}
                fn inspect(id: &'static str) {}
            }
            struct Left;
            struct Right;
            impl (Left, Right) {
                fn inspect((id, other): (&'static str, &'static str)) {}
            }
        }
    ";
    assert!(
        evaluate_identifier_policy_fixture("core-model", "control.rs", structural)
            .expect("structural identifier control must parse")
            .iter()
            .any(|finding| finding.detail.contains("Boundary::inspect"))
    );
}

/// The exemption list fails closed when an entry stops matching real source.
#[cfg(feature = "coverage-controls")]
#[test]
fn stale_exemption_entry_is_reported() {
    let findings = buoy::identifier_policy::scan_identifier_policy_fault_control(
        Path::new("."),
        "stale-exemption",
    )
    .expect("controlled scan must succeed");
    assert!(
        findings
            .iter()
            .all(|finding| finding.code == "SID-BOUNDARY-STALE-EXEMPTION"),
        "control must report only stale exemptions"
    );
    assert_eq!(findings.len(), 3);
    for fault in ["collect-entry", "nested-entry", "rust-read", "rust-syntax"] {
        assert!(
            buoy::identifier_policy::scan_identifier_policy_fault_control(Path::new("."), fault)
                .is_err(),
            "{fault}"
        );
    }

    #[cfg(unix)]
    {
        let root = tempfile::TempDir::new().expect("identifier scanner control root");
        fs::write(root.path().join("control.rs"), "fn control() {}")
            .expect("identifier scanner control source");
        std::os::unix::fs::symlink("missing", root.path().join("dangling"))
            .expect("identifier scanner dangling link");
        assert!(scan_identifier_policy(root.path()).is_ok());
    }
}
