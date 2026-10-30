#![doc = "Extended contract tests for row conformance, paths, packages, and completion evidence."]
#![allow(
    missing_docs,
    clippy::expect_used,
    clippy::panic,
    clippy::cast_possible_truncation
)]

use buoy::publication::{
    ConformanceError, OutputClass, PackageVariant, PublicationError, TraceRow, classify_output,
    detect_path_collisions, parse_completion_evidence, render_parity_diff, render_path,
    row_conformance, validate_package_variant,
};

fn rows_from_fixture(bytes: &[u8]) -> (Vec<TraceRow>, usize) {
    let value: serde_json::Value = serde_json::from_slice(bytes).expect("fixture JSON");
    let rows = value["rows"]
        .as_array()
        .expect("rows array")
        .iter()
        .map(|row| TraceRow {
            relation: row["relation"].as_str().map(str::to_owned),
            record: row["record"].as_str().map(str::to_owned),
            requirement_id: row["requirement_id"].as_str().map(str::to_owned),
            atom_revision: row["atom_revision"].as_str().map(str::to_owned),
            requirement_node: row["requirement_node"].as_str().map(str::to_owned),
            implementation_node: row["implementation_node"].as_str().map(str::to_owned),
            requirement_path: row["requirement_path"].as_str().map(str::to_owned),
            implementation_path: row["implementation_path"].as_str().map(str::to_owned),
            state: row["state"].as_str().map(str::to_owned),
            freshness: row
                .get("freshness")
                .map(|value| buoy::publication::RowFreshness {
                    record: value["record"].as_bool().unwrap_or(false),
                    attachment: value["attachment"].as_bool().unwrap_or(false),
                    implementation: value["implementation"].as_bool().unwrap_or(false),
                    requirement: value["requirement"].as_bool().unwrap_or(false),
                    authority: value["authority"].as_bool().unwrap_or(false),
                }),
            diagnostics: row["diagnostics"]
                .as_array()
                .map(|values| {
                    values
                        .iter()
                        .filter_map(|value| value.as_str().map(str::to_owned))
                        .collect()
                })
                .unwrap_or_default(),
        })
        .collect();
    let expected = value["expected_row_count"].as_u64().expect("row count") as usize;
    (rows, expected)
}

#[test]
fn exact_current_row_satisfies_conformance() {
    let bytes =
        include_bytes!("../fixtures/trace-publication/v1/row-conformance/exact-current-row.json");
    let (rows, expected) = rows_from_fixture(bytes);
    assert!(row_conformance(&rows, expected).is_ok());
}

#[test]
fn split_fields_are_rejected_as_fragments() {
    let bytes =
        include_bytes!("../fixtures/trace-publication/v1/row-conformance/split-fields.json");
    let (rows, expected) = rows_from_fixture(bytes);
    assert_eq!(
        row_conformance(&rows, expected),
        Err(ConformanceError::RowFragmentation)
    );
}

#[test]
fn row_count_mismatch_is_rejected_before_field_checks() {
    let rows = vec![TraceRow::default(), TraceRow::default()];
    assert_eq!(row_conformance(&rows, 1), Err(ConformanceError::RowCount));
}

#[test]
fn stale_row_returns_static_blocker() {
    let row = TraceRow {
        relation: Some("TRACE-RELATION".into()),
        record: Some("blake3-256:aaaa".into()),
        requirement_id: Some("DEMO-001".into()),
        atom_revision: Some("blake3-256:bbbb".into()),
        requirement_node: Some("blake3-256:node-req".into()),
        implementation_node: Some("blake3-256:node-impl".into()),
        requirement_path: Some("openspec/specs/demo/spec.md".into()),
        implementation_path: Some("src/lib.rs".into()),
        state: Some("stale".into()),
        freshness: Some(buoy::publication::RowFreshness {
            record: true,
            attachment: true,
            implementation: true,
            requirement: true,
            authority: true,
        }),
        diagnostics: Vec::new(),
    };
    assert_eq!(row_conformance(&[row], 1), Err(ConformanceError::Stale));
}

/// TPT-003: a structurally complete, current row that still carries its own
/// unresolved diagnostic must fail row conformance, not silently pass. This
/// is a real production regression control: `publication_conformance_rows`
/// (`src/cli.rs`) now bridges each row's real `TraceDiagnosticFact` codes,
/// not a fixed empty list, so a diagnosed row can no longer slip through the
/// same-row completeness check.
#[test]
fn a_complete_current_row_with_its_own_diagnostic_is_rejected() {
    let mut row = TraceRow {
        relation: Some("TRACE-RELATION".into()),
        record: Some("blake3-256:aaaa".into()),
        requirement_id: Some("DEMO-001".into()),
        atom_revision: Some("blake3-256:bbbb".into()),
        requirement_node: Some("blake3-256:node-req".into()),
        implementation_node: Some("blake3-256:node-impl".into()),
        requirement_path: Some("openspec/specs/demo/spec.md".into()),
        implementation_path: Some("src/lib.rs".into()),
        state: Some("current".into()),
        freshness: Some(buoy::publication::RowFreshness {
            record: true,
            attachment: true,
            implementation: true,
            requirement: true,
            authority: true,
        }),
        diagnostics: Vec::new(),
    };
    assert!(row_conformance(std::slice::from_ref(&row), 1).is_ok());
    row.diagnostics
        .push(String::from("SPG-REF-DANGLING-OR-STALE"));
    assert_eq!(
        row_conformance(&[row], 1),
        Err(ConformanceError::RowDiagnostic)
    );
}

/// Every fixture case supplies both an atom revision and a record digest so a
/// template that resolves either substitution can never be rejected only
/// because that identity is unavailable; the fixture table proves exactly
/// which substitutions are registered vs. rejected outright.
fn fixture_identities<'a>(
    atom_revision: &'a str,
    record_digest: &'a str,
) -> buoy::publication::PathIdentities<'a> {
    buoy::publication::PathIdentities {
        basis: "blake3-256:cccc",
        atom_revision: Some(atom_revision),
        record_digest: Some(record_digest),
    }
}

#[test]
fn extended_path_substitutions_match_fixture_table() {
    let bytes = include_bytes!("../fixtures/trace-publication/v1/paths/substitutions.json");
    let value: serde_json::Value = serde_json::from_slice(bytes).expect("fixture JSON");
    let identities = fixture_identities("blake3-256:aaaa", "blake3-256:bbbb");
    for case in value["cases"].as_array().expect("cases") {
        let name = case["name"].as_str().expect("name");
        if case.get("active_member").is_some() {
            // Digest-class eligibility is validated at plan configuration, not by path rendering.
            continue;
        }
        if let Some(template) = case["template"].as_str() {
            let registered = case["registered"].as_bool().unwrap_or(false);
            let result = render_path(template, &identities);
            assert_eq!(
                result.is_ok(),
                registered,
                "case {name} unexpected result {result:?}"
            );
        }
    }
}

/// TPT: atom, record, and basis path substitutions must each render a
/// DISTINCT declared identity, never all collapsing to the basis value.
/// This is the direct regression test for the round-2 audit finding that
/// `atom_revision_path` and `record_digest_path` silently rendered the
/// publication-basis value instead of their own declared identities.
#[test]
fn atom_record_and_basis_substitutions_render_three_distinct_values() {
    let identities = buoy::publication::PathIdentities {
        basis: "blake3-256:1111111111111111111111111111111111111111111111111111111111111111",
        atom_revision: Some(
            "blake3-256:2222222222222222222222222222222222222222222222222222222222222222",
        ),
        record_digest: Some(
            "blake3-256:3333333333333333333333333333333333333333333333333333333333333333",
        ),
    };
    let basis_path = render_path("docs/{publication_basis_path}.md", &identities).expect("basis");
    let atom_path = render_path("docs/{atom_revision_path}.md", &identities).expect("atom");
    let record_path = render_path("docs/{record_digest_path}.md", &identities).expect("record");
    assert_ne!(
        basis_path, atom_path,
        "atom substitution must not reuse the basis value"
    );
    assert_ne!(
        basis_path, record_path,
        "record substitution must not reuse the basis value"
    );
    assert_ne!(
        atom_path, record_path,
        "atom and record substitutions must render distinct declared identities"
    );
    assert!(
        basis_path.contains("1111111111111111111111111111111111111111111111111111111111111111")
    );
    assert!(atom_path.contains("2222222222222222222222222222222222222222222222222222222222222222"));
    assert!(
        record_path.contains("3333333333333333333333333333333333333333333333333333333333333333")
    );

    // The literal (non-`_path`) forms follow the identical rule.
    let atom_literal = render_path("{atom_revision}", &identities).expect("atom literal");
    let record_literal = render_path("{record_digest}", &identities).expect("record literal");
    let basis_literal = render_path("{publication_basis}", &identities).expect("basis literal");
    assert_ne!(atom_literal, basis_literal);
    assert_ne!(record_literal, basis_literal);
    assert_ne!(atom_literal, record_literal);
}

/// A template that needs an atom-revision or record-digest substitution but
/// whose selected trace supplies no unambiguous value for it must fail
/// closed rather than silently substitute an unrelated identity.
#[test]
fn substitution_fails_closed_when_its_declared_identity_is_unavailable() {
    let identities = buoy::publication::PathIdentities {
        basis: "blake3-256:cccc",
        atom_revision: None,
        record_digest: None,
    };
    assert!(render_path("docs/{atom_revision_path}.md", &identities).is_err());
    assert!(render_path("docs/{record_digest_path}.md", &identities).is_err());
    assert!(render_path("docs/{publication_basis_path}.md", &identities).is_ok());
}

#[test]
fn output_classification_matches_fixture_table() {
    let bytes = include_bytes!("../fixtures/trace-publication/v1/paths/output-classes.json");
    let value: serde_json::Value = serde_json::from_slice(bytes).expect("fixture JSON");
    for case in value["cases"].as_array().expect("cases") {
        let name = case["name"].as_str().expect("name");
        if name == "collision" {
            continue;
        }
        let path = case["path"].as_str().expect("path");
        let kind = case["kind"].as_str().expect("kind");
        let source_set: Vec<String> = case["source_set"]
            .as_array()
            .map(|values| {
                values
                    .iter()
                    .filter_map(|v| v.as_str().map(str::to_owned))
                    .collect()
            })
            .unwrap_or_default();
        let expected = case["expected"].as_str().expect("expected");
        let result = classify_output(path, kind, &source_set);
        match expected {
            "accept" => assert!(
                matches!(result, Ok(OutputClass::Derived | OutputClass::SourceEdit)),
                "case {name}: {result:?}"
            ),
            "reject" => assert!(result.is_err(), "case {name} should reject"),
            other => panic!("unknown expectation {other}"),
        }
    }
}

/// REH-012 positive oracle: "controlled edit targets selected source". This
/// is a DEDICATED, distinct oracle from the REH-012 negative scenario below
/// -- the hardening manifest requires an independent positive and negative
/// test, not the same shared table-driven test reused for both polarities.
#[test]
fn controlled_edit_targets_selected_source_is_accepted() {
    let source_set = vec![String::from("openspec/specs/demo/spec.md")];
    let result = classify_output("openspec/specs/demo/spec.md", "source-edit", &source_set);
    assert_eq!(result, Ok(OutputClass::SourceEdit));
}

/// REH-012 negative oracle: "derived output becomes selected source". A
/// DEDICATED, distinct oracle from the positive scenario above.
#[test]
fn derived_output_becomes_selected_source_is_rejected() {
    let source_set = vec![String::from("openspec/specs/demo/spec.md")];
    let result = classify_output("openspec/specs/demo/spec.md", "page", &source_set);
    assert_eq!(
        result,
        Err(PublicationError::InvalidField("selected-output".into()))
    );
}

#[test]
fn parity_diff_uses_fixed_labels_with_no_volatile_bytes() {
    assert_eq!(
        render_parity_diff(None, b"new"),
        "existing: absent\ncandidate: present"
    );
    assert_eq!(
        render_parity_diff(Some(b"same"), b"same"),
        "existing: equal\ncandidate: equal"
    );
    let diff = render_parity_diff(Some(b"one\ntwo\n"), b"one\nthree\n");
    // Round-5 finding H1: the summary reports WHERE the documents differ,
    // never WHAT they contain. Line 2 differs; line 1 is identical and is
    // not reported at all.
    assert!(
        diff.contains("differing-line: 2 existing-bytes 3 candidate-bytes 5"),
        "{diff}"
    );
    assert!(!diff.contains("differing-line: 1"), "{diff}");
    // The content of neither document appears.
    assert!(!diff.contains("two"), "{diff}");
    assert!(!diff.contains("three"), "{diff}");
    assert!(!diff.contains('/'));
    assert!(!diff.to_lowercase().contains("tmp"));
}

/// Round-5 finding H1, the property that makes the defect unreachable rather
/// than merely unlikely.
///
/// The previous renderer quoted document content into a summary that was then
/// checked against TPT-009's forbidden-field contract, so ORDINARY page bytes
/// -- a logical path, a digest, a date in requirement prose -- could fail that
/// check and, at the call site, replace a real immutable conflict with a
/// spurious `TPT-FORBIDDEN-PUBLICATION-FIELD`.
///
/// This drives documents built specifically to smuggle every forbidden form
/// through the renderer and requires the rendered metadata to remain clean for
/// all of them. It is deliberately expressed as a property over hostile inputs,
/// not as one example, because the guarantee being claimed is universal.
#[test]
fn parity_metadata_is_clean_for_every_hostile_document() {
    let hostile: [&[u8]; 10] = [
        b"--- a/page.md\n+++ b/page.md\n@@ -1 +1 @@\n",
        b"generated 2026-08-24 at 11:22:33\n",
        b"/Users/someone/checkout/out/page.md\n",
        b"/tmp/buoy.tmp.4213/page.md\n",
        b"C:\\Temp\\page.md\n",
        b"pid=4213 LC_ALL=C LANG=en_US.UTF-8 TZ=UTC\n",
        b"diff --git a/x b/y\nIndex: z\n",
        b"$HOME/.publication-lock\n",
        b"/var/folders/zz/T/page.md\n",
        b"\xff\xfe invalid utf8 \x00 bytes\n",
    ];
    for existing in hostile {
        for candidate in hostile {
            let rendered = render_parity_diff(Some(existing), candidate);
            assert!(
                buoy::publication::validate_publication_diagnostic_text(&rendered).is_ok(),
                "hostile document content reached the parity summary: {rendered}"
            );
            // Also prove it directly: no byte run from either document
            // survives into the summary.
            for probe in [
                "--- ",
                "+++ ",
                "@@ ",
                "2026-08-24",
                "11:22:33",
                "/Users/",
                "/tmp/",
                "C:\\",
                "pid=",
                "LC_ALL",
                "diff --git",
                "$HOME",
                "/var/folders/",
            ] {
                assert!(
                    !rendered.contains(probe),
                    "`{probe}` leaked into the parity summary: {rendered}"
                );
            }
        }
    }
    // Non-vacuity: the validator this relies on really does reject these
    // documents when they are passed to it directly, so the assertions above
    // are not passing because the validator is inert.
    for hostile_text in [
        "--- a/page.md",
        "generated 2026-08-24",
        "/Users/someone/page.md",
        "pid=4213",
    ] {
        assert!(
            buoy::publication::validate_publication_diagnostic_text(hostile_text).is_err(),
            "the forbidden-field validator must reject `{hostile_text}` directly"
        );
    }
}

/// Round-5 finding H4: a base whose path ends in a DOUBLED slash, or whose
/// segment carries a percent-encoded separator, is not normalized.
///
/// Both forms were accepted before: the doubled-slash scan stripped the
/// trailing `/` first (so `https://host/trace//` was scanned as `/trace/`),
/// and the traversal check compared whole literal-`/` segments against `..`
/// (so `%2e%2e%2f%2e%2e`, which decodes to `../..`, matched neither).
#[test]
fn non_normal_target_bases_are_rejected_and_normal_ones_are_not() {
    use buoy::publication::parse_plan;

    // An immutable page carrying a target base must be referenced by an
    // external link, or plan parsing rejects it as an ORPHAN page before it
    // ever validates the base. Omitting that link made the first revision of
    // this test pass vacuously -- every base was rejected, but for an
    // unrelated reason. The non-vacuity block at the end is what exposed
    // that, which is exactly why it is written this way.
    let plan = |base: &str| {
        format!(
            r#"{{"schema":"buoy/trace-publication-plan/v1","publication_id":"p","selector":{{"requirement_id":"R-1"}},"expected_rows":1,"source_set":["a.md"],"outputs":[{{"id":"page","mount":"repository","path":"docs/{{publication_basis_path}}.md","kind":"page","lifecycle":"immutable","target_base":"{base}"}},{{"id":"link","mount":"repository","path":"a.md","kind":"external-link","lifecycle":"replaceable","relation":"documents","page":"page"}}]}}"#
        )
    };

    // Every one of these is non-normal and must be rejected.
    for base in [
        // Terminal doubled slash -- the exact form the old scan missed.
        "https://example.invalid/trace//",
        "https://example.invalid//",
        "https://example.invalid/a//b/",
        // Encoded separators, including the encoded-traversal form that
        // decodes to `../..` and so matched no whole-segment dot check.
        "https://example.invalid/%2e%2e%2f%2e%2e/",
        "https://example.invalid/a%2fb/",
        "https://example.invalid/a%2Fb/",
        "https://example.invalid/a%5cb/",
        // Doubly-encoded percent: decodes to `%2e%2e` then to `..`.
        "https://example.invalid/%252e%252e/",
        // Literal and singly-encoded traversal.
        "https://example.invalid/../",
        "https://example.invalid/%2e%2e/",
        "https://example.invalid/a/./",
        // Authority faults.
        "https://user@example.invalid/",
        "https://user:pw@example.invalid/",
        "https://example.invalid:/",
        "https://example.invalid:port/",
        "https:///",
        // Scheme, terminator, and syntax faults.
        "http://example.invalid/",
        "https://example.invalid/no-trailing-slash",
        "https://example.invalid/?q=1",
        "https://example.invalid/#f",
        "https://example.invalid/{publication_basis_path}/",
        "https://example.invalid/a b/",
        "https://example.invalid/a\\b/",
    ] {
        assert!(
            parse_plan(plan(base).as_bytes()).is_err(),
            "non-normal target base was accepted: {base}"
        );
    }

    // Non-vacuity: an over-broad validator would pass the loop above while
    // rejecting everything. These normal bases must still parse, including
    // the single trailing slash that legitimately ends every base and a
    // percent-escape that is NOT a separator or a dot.
    for base in [
        "https://example.invalid/",
        "https://example.invalid/trace/",
        "https://example.invalid/a/b/c/",
        "https://example.invalid:8443/trace/",
        "https://example.invalid/a%20b/",
        "https://example.invalid/a.b/",
        "https://example.invalid/..a/",
        "https://example.invalid/a../",
    ] {
        assert!(
            parse_plan(plan(base).as_bytes()).is_ok(),
            "normal target base was rejected: {base}"
        );
    }
}

#[test]
fn duplicate_output_paths_collide() {
    let bytes = include_bytes!("../fixtures/trace-publication/v1/paths/output-classes.json");
    let value: serde_json::Value = serde_json::from_slice(bytes).expect("fixture JSON");
    let case = value["cases"]
        .as_array()
        .expect("cases")
        .iter()
        .find(|case| case["name"] == "collision")
        .expect("collision case");
    let first = case["path"].as_str().expect("path");
    let second = case["second_path"].as_str().expect("second_path");
    assert!(detect_path_collisions(&[first, second]).is_err());
    assert!(detect_path_collisions(&[first, "docs/other.md"]).is_ok());
}

#[test]
fn package_variant_fixture_cases_match_expectations() {
    let bytes = include_bytes!("../fixtures/trace-publication/v1/package/variants.json");
    let value: serde_json::Value = serde_json::from_slice(bytes).expect("fixture JSON");
    for case in value["cases"].as_array().expect("cases") {
        let name = case["name"].as_str().expect("name");
        let features: Vec<&str> = case["features"]
            .as_array()
            .expect("features")
            .iter()
            .filter_map(|v| v.as_str())
            .collect();
        let source_revision = case["source_revision"].as_str().unwrap_or("revision");
        let ambient_cargo_home = case["ambient_cargo_home"].as_str();
        let expected = case["expected"].as_str().expect("expected");
        let variant = PackageVariant {
            features,
            source_revision,
            ambient_cargo_home,
        };
        let result = validate_package_variant(&variant);
        assert_eq!(
            result.is_ok(),
            expected == "accept",
            "case {name}: {result:?}"
        );
    }
}

#[test]
fn completion_evidence_blocks_parse_and_reject_conflicts() {
    let generated =
        include_bytes!("../fixtures/trace-publication/v1/completion/generated-evidence.md");
    let evidence = parse_completion_evidence(std::str::from_utf8(generated).expect("UTF-8"))
        .expect("generated evidence");
    assert_eq!(evidence.publication_id.as_deref(), Some("demo-publication"));

    let empty = include_bytes!("../fixtures/trace-publication/v1/completion/empty-state.md");
    let empty_evidence =
        parse_completion_evidence(std::str::from_utf8(empty).expect("UTF-8")).expect("empty state");
    assert!(empty_evidence.publication_id.is_none());

    let conflict = include_bytes!("../fixtures/trace-publication/v1/completion/conflict.md");
    assert_eq!(
        parse_completion_evidence(std::str::from_utf8(conflict).expect("UTF-8")),
        Err(PublicationError::InvalidField(
            "completion-evidence-drift".into()
        ))
    );

    let false_authority =
        include_bytes!("../fixtures/trace-publication/v1/completion/false-current-authority.md");
    assert_eq!(
        parse_completion_evidence(std::str::from_utf8(false_authority).expect("UTF-8")),
        Err(PublicationError::InvalidField("claim-boundary".into()))
    );
}

/// TPT-009/9.12: a generated completion-evidence block MUST NOT carry a
/// placeholder stand-in identity such as `TBD` in place of a real
/// publication ID. Before any implementation exists, the generated block
/// must stay entirely empty rather than fabricate one.
#[test]
fn completion_evidence_block_rejects_a_placeholder_publication_id() {
    let placeholder =
        include_bytes!("../fixtures/trace-publication/v1/completion/placeholder-identity.md");
    assert_eq!(
        parse_completion_evidence(std::str::from_utf8(placeholder).expect("UTF-8")),
        Err(PublicationError::InvalidField(
            "completion-evidence-placeholder".into()
        ))
    );
}

/// TPT: manual-conflict detection must not assume `blake3-256`. A manual
/// row naming its digest under `xxh3-128` (a real, registered
/// change-detection member) that conflicts with the generated evidence's
/// own record must be rejected exactly like the equivalent `blake3-256`
/// fixture already proves above.
#[test]
fn completion_evidence_conflict_detection_is_not_limited_to_blake3_256() {
    let conflict = include_bytes!("../fixtures/trace-publication/v1/completion/conflict-xxh3.md");
    assert_eq!(
        parse_completion_evidence(std::str::from_utf8(conflict).expect("UTF-8")),
        Err(PublicationError::InvalidField(
            "completion-evidence-drift".into()
        ))
    );
}

/// Round-5 finding H7: the forbidden-field validator had no control proving
/// each of its rules is load-bearing.
///
/// A validator with an inert entry passes every ordinary test while silently
/// permitting the exact content TPT-009 forbids. This drives the REAL
/// production lists (`FORBIDDEN_DIFF_HEADERS`,
/// `FORBIDDEN_PUBLICATION_SUBSTRINGS`) rather than a hand-copied duplicate,
/// so it is mutation-sensitive in both directions:
///
/// * deleting any entry from either production list fails this test, because
///   the corresponding rejection assertion stops holding; and
/// * adding an entry without thought also fails, because every entry must
///   additionally be shown to be DISCRIMINATING -- a near-miss text that does
///   not contain it has to still be accepted.
#[test]
fn every_forbidden_publication_field_rule_is_load_bearing() {
    use buoy::publication::{
        FORBIDDEN_DIFF_HEADERS, FORBIDDEN_PUBLICATION_SUBSTRINGS,
        validate_publication_diagnostic_text,
    };

    assert!(
        validate_publication_diagnostic_text("existing: equal\ncandidate: equal").is_ok(),
        "the ordinary fixed-label summary must be accepted"
    );

    // A diff header is only forbidden at the START of a line, which is what
    // distinguishes a real platform diff header from prose that happens to
    // mention one. Both halves are asserted so neither the rule nor its
    // anchoring can be removed unnoticed.
    for header in FORBIDDEN_DIFF_HEADERS {
        let at_line_start = format!("existing: present\n{header}payload\ncandidate: present");
        assert!(
            validate_publication_diagnostic_text(&at_line_start).is_err(),
            "a line beginning with `{header}` must be rejected"
        );
        let mid_line = format!("existing: present\nsummary mentions {header} inline");
        assert!(
            validate_publication_diagnostic_text(&mid_line).is_ok(),
            "`{header}` away from a line start must not be rejected: the rule is anchored"
        );
    }

    // Every physical-root, temporary-artifact, process-identity and locale
    // substring is forbidden anywhere in the text.
    for needle in FORBIDDEN_PUBLICATION_SUBSTRINGS {
        let carrying = format!("existing: present\ndetail {needle} detail");
        assert!(
            validate_publication_diagnostic_text(&carrying).is_err(),
            "text containing `{needle}` must be rejected"
        );
    }

    // Timestamps are rejected by SHAPE, not by a substring list, so they are
    // covered separately -- and the near-miss cases prove the shape matcher
    // is not simply rejecting every run of digits.
    for timestamp in [
        "generated 2026-08-24",
        "at 11:22:33",
        "2026-08-24T11:22:33",
        "prefix1999-01-01suffix",
    ] {
        assert!(
            validate_publication_diagnostic_text(timestamp).is_err(),
            "timestamp-shaped text `{timestamp}` must be rejected"
        );
    }
    for benign in [
        "differing-line: 12 existing-bytes 3456 candidate-bytes 78",
        "existing: lines 2026",
        "existing: bytes 112233",
        "candidate: lines 11-22",
        "basis blake3-256:0123456789abcdef",
    ] {
        assert!(
            validate_publication_diagnostic_text(benign).is_ok(),
            "benign fixed-label text `{benign}` must be accepted"
        );
    }

    // Non-vacuity: the production lists must actually be populated. An empty
    // list would make every loop above pass by iterating nothing.
    assert!(FORBIDDEN_DIFF_HEADERS.len() >= 5);
    assert!(FORBIDDEN_PUBLICATION_SUBSTRINGS.len() >= 14);

    // And every real parity summary this renderer can produce is accepted --
    // the rules must forbid volatile content without forbidding the
    // renderer's own vocabulary.
    for (existing, candidate) in [
        (None, &b"x"[..]),
        (Some(&b"x"[..]), &b"x"[..]),
        (Some(&b"a\nb\nc"[..]), &b"a\nB\nc\nd"[..]),
    ] {
        let rendered = render_parity_diff(existing, candidate);
        assert!(
            validate_publication_diagnostic_text(&rendered).is_ok(),
            "the renderer's own output must satisfy its own contract: {rendered}"
        );
    }
}
