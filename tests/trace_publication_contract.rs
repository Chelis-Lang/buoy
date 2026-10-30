#![doc = "Contract tests for typed trace publication plans."]
#![allow(missing_docs, clippy::expect_used)]

use buoy::check::ShellDiagnostic;
use buoy::publication::{
    BasisInputs, PublicationError, Selector, TraceRow, compute_publication_basis,
    omit_declared_link_diagnostics, parse_plan,
};
use std::fmt::Write as _;

#[test]
fn complete_plan_parses_into_closed_types() {
    let bytes = include_bytes!("../fixtures/trace-publication/v1/plan-positive.json");
    let plan = parse_plan(bytes).expect("positive publication plan");
    assert_eq!(plan.schema(), "buoy/trace-publication-plan/v1");
    assert_eq!(plan.publication_id(), "demo-publication");
    assert_eq!(plan.outputs().len(), 2);
    assert_eq!(plan.immutable_page_count(), 1);
}

/// TPT-001: "A page can declare one normalized absolute target base." An
/// external-link output only references a declared page by ID; it never
/// owns or overrides the complete target itself. A `target_base` declared
/// on a non-page output must fail plan parsing.
#[test]
fn target_base_declared_on_a_non_page_output_fails_plan_parsing() {
    let bytes = br#"{"schema":"buoy/trace-publication-plan/v1","publication_id":"demo","selector":{"requirement_id":"DEMO-001"},"expected_rows":1,"source_set":[],"outputs":[{"id":"trace-page","mount":"repository","path":"docs/trace/{publication_basis_path}.md","kind":"page","lifecycle":"replaceable"},{"id":"link","mount":"repository","path":"openspec/specs/demo/spec.md","kind":"external-link","lifecycle":"replaceable","relation":"documents","page":"trace-page","target_base":"https://example.invalid/trace/"}]}"#;
    assert_eq!(
        parse_plan(bytes),
        Err(PublicationError::InvalidField(
            "target-base-not-page-owned".into()
        ))
    );
}

/// The corrected fixture (target base declared on the page, not the link)
/// parses cleanly, proving the ownership fix does not merely reject the old
/// shape but accepts the correctly typed one.
#[test]
fn target_base_declared_on_the_page_output_parses_cleanly() {
    let bytes = include_bytes!("../fixtures/trace-publication/v1/plan-positive.json");
    let plan = parse_plan(bytes).expect("positive publication plan with page-owned target base");
    let page = plan
        .outputs()
        .iter()
        .find(|output| output.id == "trace-page")
        .expect("declared page output");
    assert_eq!(
        page.target_base.as_deref(),
        Some("https://example.invalid/trace/")
    );
    let link = plan
        .outputs()
        .iter()
        .find(|output| output.id == "external-link")
        .expect("declared link output");
    assert!(link.target_base.is_none());
}

#[test]
fn unknown_fields_fail_before_materialization() {
    let bytes = include_bytes!("../fixtures/trace-publication/v1/plan-unknown-field.json");
    assert_eq!(
        parse_plan(bytes),
        Err(PublicationError::UnknownField("unexpected".into()))
    );
}

#[test]
fn malformed_plan_reports_input_class() {
    assert_eq!(
        parse_plan(b"{}"),
        Err(PublicationError::MissingField("schema".into()))
    );
    assert_eq!(
        PublicationError::MissingField("schema".into()).exit_class(),
        70
    );
}

#[test]
fn conflicting_selector_forms_return_the_usage_exit_class() {
    let bytes = include_bytes!("../fixtures/trace-publication/v1/plan-conflicting-selector.json");
    let error = parse_plan(bytes).expect_err("two selector fields must conflict");
    assert_eq!(error, PublicationError::ConflictingSelector);
    assert_eq!(error.exit_class(), 64, "TPT-002 requires usage exit 64");
}

#[test]
fn immutable_page_with_no_referencing_link_is_an_orphan() {
    let bytes = include_bytes!("../fixtures/trace-publication/v1/plan-orphan-immutable-page.json");
    let error = parse_plan(bytes).expect_err("an unreferenced immutable page must be rejected");
    assert_eq!(error, PublicationError::OrphanImmutablePage);
    assert_eq!(error.exit_class(), 70, "TPT-001 requires input exit 70");
}

#[test]
fn external_link_page_reference_must_resolve_to_a_declared_page_output() {
    let bytes =
        include_bytes!("../fixtures/trace-publication/v1/plan-external-link-unresolved-page.json");
    let error = parse_plan(bytes).expect_err("a dangling page reference must be rejected");
    assert_eq!(
        error,
        PublicationError::InvalidField("external-link-unresolved-page".into())
    );
    assert_eq!(error.exit_class(), 70);
}

#[test]
fn two_external_links_cannot_claim_the_same_page_output() {
    let bytes =
        include_bytes!("../fixtures/trace-publication/v1/plan-external-link-multiple-owners.json");
    let error = parse_plan(bytes).expect_err("two links claiming one page must be rejected");
    assert_eq!(
        error,
        PublicationError::InvalidField("external-link-multiple-owners".into())
    );
    assert_eq!(error.exit_class(), 70);
}

/// TPT-006: version 1 accepts only relation `documents` for a
/// publication-managed external-link output; any other relation fails plan
/// parsing before any repository operation runs.
#[test]
fn external_link_output_rejects_any_relation_other_than_documents() {
    let bytes =
        include_bytes!("../fixtures/trace-publication/v1/plan-external-link-wrong-relation.json");
    let error = parse_plan(bytes).expect_err("a non-documents relation must be rejected");
    assert_eq!(
        error,
        PublicationError::InvalidField("external-link relation".into())
    );
    assert_eq!(error.exit_class(), 70);
}

fn identities(basis: &str) -> buoy::publication::PathIdentities<'_> {
    buoy::publication::PathIdentities {
        basis,
        atom_revision: None,
        record_digest: None,
    }
}

#[test]
fn registered_path_substitutions_are_deterministic() {
    assert_eq!(
        buoy::publication::render_path(
            "docs/{publication_basis_path}.md",
            &identities("blake3-256:abc")
        )
        .expect("path"),
        "docs/blake3-256/abc.md"
    );
}

#[test]
fn unregistered_path_substitution_is_rejected() {
    assert_eq!(
        buoy::publication::render_path("docs/{page_commit}.md", &identities("x")),
        Err(PublicationError::InvalidField("substitution".into()))
    );
}

/// TPT/3.5: an unregistered substitution (or an unregistered substitution
/// combined with any other kind) must fail closed at PLAN PARSING itself,
/// not only the first time an operation later renders the path. This is
/// the real production-boundary proof: `parse_plan` -- the strict typed
/// boundary every operation calls before any repository read -- must reject
/// it directly.
#[test]
fn plan_parsing_rejects_an_unregistered_substitution_before_any_repository_operation() {
    for (name, path) in [
        ("graph", "docs/{graph_digest}.md"),
        ("page", "docs/{page_digest}.md"),
        ("link", "docs/{link_digest}.md"),
        ("commit", "docs/{page_commit}.md"),
        ("provider", "docs/{provider_id}.md"),
        ("unknown", "docs/{mystery}.md"),
    ] {
        let bytes = format!(
            r#"{{"schema":"buoy/trace-publication-plan/v1","publication_id":"demo","selector":{{"requirement_id":"DEMO-001"}},"expected_rows":1,"source_set":[],"outputs":[{{"id":"trace-page","mount":"repository","path":"{path}","kind":"page","lifecycle":"replaceable"}}]}}"#
        );
        assert_eq!(
            parse_plan(bytes.as_bytes()),
            Err(PublicationError::InvalidField("substitution".into())),
            "case {name}"
        );
    }
}

/// A `source-edit` or `external-link` output path never uses templating at
/// all; a literal `{`/`}` in one of those paths must fail plan parsing
/// rather than be silently treated as an ordinary path character or wait
/// for a later, unrelated failure.
#[test]
fn a_substitution_brace_in_a_source_edit_or_external_link_path_fails_plan_parsing() {
    let bytes = br#"{"schema":"buoy/trace-publication-plan/v1","publication_id":"demo","selector":{"requirement_id":"DEMO-001"},"expected_rows":1,"source_set":[],"outputs":[{"id":"edit","mount":"repository","path":"openspec/specs/demo/{spec}.md","kind":"source-edit","lifecycle":"replaceable"}]}"#;
    assert_eq!(
        parse_plan(bytes),
        Err(PublicationError::InvalidField(
            "output-path-no-substitution".into()
        ))
    );
}

#[test]
fn publication_digest_domains_are_distinct() {
    let bytes = b"same";
    let algorithm = buoy::core::DigestAlgorithm::Xxh3_128;
    assert_ne!(
        buoy::publication::publication_basis_digest(algorithm, bytes).expect("basis digest"),
        buoy::publication::page_content_digest(algorithm, bytes).expect("content digest")
    );
}

#[test]
fn publication_digest_uses_the_real_registry_member() {
    // The digest must come from the same repository digest registry every
    // other Buoy identity uses, not a private hash: this asserts equality
    // against `ActiveAlgorithm::compute` directly, over the same domain-tagged
    // preimage `publication_basis_digest` constructs internally.
    let algorithm = buoy::core::DigestAlgorithm::Xxh3_128;
    let active = buoy::core::ActiveAlgorithm::new(algorithm).expect("xxh3-128 always available");
    let mut preimage = b"buoy/trace-publication-basis/v1".to_vec();
    preimage.extend_from_slice(b"same");
    let expected = active
        .compute(algorithm, &preimage)
        .expect("registry compute")
        .canonical_text();
    assert_eq!(
        buoy::publication::publication_basis_digest(algorithm, b"same").expect("basis digest"),
        expected
    );
}

#[test]
fn plan_bytes_beyond_the_budget_fail_before_allocation() {
    let oversized = vec![b'a'; buoy::publication::PLAN_BYTES_BUDGET + 1];
    assert_eq!(
        buoy::publication::parse_plan(&oversized),
        Err(PublicationError::PlanBytesBudget)
    );
}

#[test]
fn plan_output_count_beyond_the_budget_fails_closed() {
    let mut outputs = String::new();
    for index in 0..=buoy::publication::PLAN_OUTPUT_BUDGET {
        if index > 0 {
            outputs.push(',');
        }
        let _ = write!(
            outputs,
            r#"{{"id":"output-{index}","mount":"repository","path":"docs/{index}.md","kind":"page","lifecycle":"replaceable"}}"#
        );
    }
    let plan = format!(
        r#"{{"schema":"buoy/trace-publication-plan/v1","publication_id":"budget","selector":{{"requirement_id":"DEMO-001"}},"expected_rows":1,"source_set":[],"outputs":[{outputs}]}}"#
    );
    assert_eq!(
        buoy::publication::parse_plan(plan.as_bytes()),
        Err(PublicationError::PlanOutputBudget)
    );
}

#[test]
fn plan_source_set_count_beyond_the_budget_fails_closed() {
    let mut entries = String::new();
    for index in 0..=buoy::publication::PLAN_SOURCE_SET_BUDGET {
        if index > 0 {
            entries.push(',');
        }
        let _ = write!(entries, "\"path-{index}.md\"");
    }
    let plan = format!(
        r#"{{"schema":"buoy/trace-publication-plan/v1","publication_id":"budget","selector":{{"requirement_id":"DEMO-001"}},"expected_rows":1,"source_set":[{entries}],"outputs":[]}}"#
    );
    assert_eq!(
        buoy::publication::parse_plan(plan.as_bytes()),
        Err(PublicationError::PlanSourceSetBudget)
    );
}

#[test]
fn plan_within_every_budget_still_parses() {
    let bytes = include_bytes!("../fixtures/trace-publication/v1/plan-positive.json");
    assert!(buoy::publication::parse_plan(bytes).is_ok());
}

#[test]
fn documented_generic_mounted_example_plan_parses() {
    // This is the exact plan documented in
    // docs/schema/v3/trace-publication-tooling.md under "Generic mounted
    // example". It must always parse: it is a live worked example, not
    // aspirational text.
    let plan = br#"{
  "schema": "buoy/trace-publication-plan/v1",
  "publication_id": "consumer-example",
  "selector": { "requirement_id": "EXAMPLE-001" },
  "expected_rows": 1,
  "source_set": ["docs/spec.md"],
  "outputs": [
    {
      "id": "trace-page",
      "mount": "repository",
      "path": "docs/trace/current.md",
      "kind": "page",
      "lifecycle": "replaceable"
    }
  ]
}"#;
    let parsed = buoy::publication::parse_plan(plan).expect("documented example must parse");
    assert_eq!(parsed.publication_id(), "consumer-example");
}

fn sample_row() -> TraceRow {
    TraceRow {
        relation: Some(String::from("documents")),
        record: Some(String::from("blake3-256:aaaa")),
        requirement_id: Some(String::from("DEMO-001")),
        atom_revision: Some(String::from("blake3-256:bbbb")),
        requirement_node: Some(String::from("blake3-256:node-req")),
        implementation_node: Some(String::from("blake3-256:node-impl")),
        requirement_path: Some(String::from("openspec/specs/demo/spec.md")),
        implementation_path: Some(String::from("src/lib.rs")),
        state: Some(String::from("current")),
        freshness: Some(buoy::publication::RowFreshness {
            record: true,
            attachment: true,
            implementation: true,
            requirement: true,
            authority: true,
        }),
        diagnostics: Vec::new(),
    }
}

#[test]
fn publication_basis_is_independent_of_snapshot_ordering_under_a_fixed_input_ordering() {
    // The basis encoder is order-sensitive only in the caller-supplied
    // normalized-snapshot ordering, so a caller that sorts the snapshot by
    // path before calling it (as every production caller must) always gets
    // one deterministic result regardless of selector discovery order.
    let selector = Selector::RequirementId(String::from("DEMO-001"));
    let rows = vec![sample_row()];
    let mut snapshot = vec![
        (
            String::from("openspec/specs/demo/spec.md"),
            b"# Demo".to_vec(),
        ),
        (String::from("src/lib.rs"), b"pub fn demo() {}".to_vec()),
    ];
    let first = compute_publication_basis(&BasisInputs {
        plan_schema: "buoy/trace-publication-plan/v1",
        renderer_schema: "buoy/readable-trace-markdown/v1",
        active_member: "xxh3-128",
        selector: &selector,
        trace_rows: &rows,
        normalized_snapshot: &snapshot,
    })
    .expect("basis computes");

    // Sort into a different discovery order, then re-sort back to the same
    // canonical order: the basis must be reproducible from equal sorted
    // input regardless of what order the filesystem originally returned it.
    snapshot.reverse();
    snapshot.sort_by(|a, b| a.0.cmp(&b.0));
    let second = compute_publication_basis(&BasisInputs {
        plan_schema: "buoy/trace-publication-plan/v1",
        renderer_schema: "buoy/readable-trace-markdown/v1",
        active_member: "xxh3-128",
        selector: &selector,
        trace_rows: &rows,
        normalized_snapshot: &snapshot,
    })
    .expect("basis computes again");
    assert_eq!(first, second);
    assert!(first.starts_with("blake3-256:") || first.starts_with("xxh3-128:"));
}

#[test]
fn publication_basis_rejects_an_incomplete_trace_row() {
    let selector = Selector::RequirementId(String::from("DEMO-001"));
    let mut row = sample_row();
    row.atom_revision = None;
    let snapshot = Vec::new();
    assert_eq!(
        compute_publication_basis(&BasisInputs {
            plan_schema: "buoy/trace-publication-plan/v1",
            renderer_schema: "buoy/readable-trace-markdown/v1",
            active_member: "xxh3-128",
            selector: &selector,
            trace_rows: &[row],
            normalized_snapshot: &snapshot,
        }),
        Err(PublicationError::InvalidField(
            "incomplete trace row in basis input".into()
        ))
    );
}

// This test proves the basis changes when the active digest *member*
// changes, which structurally requires two distinct compiled algorithms
// (the default `xxh3-128` plus `sha-256`); it therefore needs this second
// feature explicitly, unlike every other test in this file.
#[cfg(feature = "sha-256")]
#[test]
fn publication_basis_changes_when_any_bound_input_changes() {
    let selector = Selector::RequirementId(String::from("DEMO-001"));
    let rows = vec![sample_row()];
    let snapshot = vec![(String::from("src/lib.rs"), b"pub fn demo() {}".to_vec())];
    let base = compute_publication_basis(&BasisInputs {
        plan_schema: "buoy/trace-publication-plan/v1",
        renderer_schema: "buoy/readable-trace-markdown/v1",
        active_member: "xxh3-128",
        selector: &selector,
        trace_rows: &rows,
        normalized_snapshot: &snapshot,
    })
    .expect("base basis");

    let changed_bytes = vec![(String::from("src/lib.rs"), b"pub fn demo() { }".to_vec())];
    let with_changed_bytes = compute_publication_basis(&BasisInputs {
        plan_schema: "buoy/trace-publication-plan/v1",
        renderer_schema: "buoy/readable-trace-markdown/v1",
        active_member: "xxh3-128",
        selector: &selector,
        trace_rows: &rows,
        normalized_snapshot: &changed_bytes,
    })
    .expect("changed-bytes basis");
    assert_ne!(base, with_changed_bytes);

    let with_changed_member = compute_publication_basis(&BasisInputs {
        plan_schema: "buoy/trace-publication-plan/v1",
        renderer_schema: "buoy/readable-trace-markdown/v1",
        active_member: "sha-256",
        selector: &selector,
        trace_rows: &rows,
        normalized_snapshot: &snapshot,
    })
    .expect("changed-member basis");
    assert_ne!(base, with_changed_member);
}

fn diagnostic(code: &str, id: Option<&str>) -> ShellDiagnostic {
    ShellDiagnostic {
        code: code.to_owned(),
        path: None,
        start: 0,
        end: 0,
        id: id.map(str::to_owned),
        context: String::new(),
    }
}

#[test]
fn pre_link_conformance_omits_only_the_declared_links_own_diagnostics() {
    // Task 8.5: a stale or absent reference for the exact link about to be
    // regenerated must not block that generation, but every other
    // diagnostic (a different link, a relation, or an authority fact) must
    // remain a blocker.
    let diagnostics = vec![
        diagnostic("SPG-REF-DANGLING-OR-STALE", Some("demo-link")),
        diagnostic("SPG-REF-DANGLING-OR-STALE", Some("other-link")),
        diagnostic("SPG-AUTHORITY-CONFLICT", None),
    ];
    let filtered = omit_declared_link_diagnostics(&diagnostics, "demo-link");
    assert_eq!(filtered.len(), 2);
    assert!(
        filtered
            .iter()
            .all(|diagnostic| diagnostic.id.as_deref() != Some("demo-link"))
    );
    assert!(
        filtered
            .iter()
            .any(|diagnostic| diagnostic.id.as_deref() == Some("other-link"))
    );
    assert!(filtered.iter().any(|diagnostic| diagnostic.id.is_none()));
}

#[test]
fn pre_link_conformance_omits_nothing_when_no_diagnostic_names_the_link() {
    let diagnostics = vec![diagnostic("SPG-AUTHORITY-CONFLICT", None)];
    let filtered = omit_declared_link_diagnostics(&diagnostics, "demo-link");
    assert_eq!(filtered, diagnostics);
}

#[test]
fn immutable_page_path_using_only_the_atom_revision_fails_the_plan_boundary() {
    // Task 8.17: an atom-only (or otherwise constant) key must never satisfy
    // an immutable page path; only `{publication_basis_path}` does. This
    // complements `render_path`'s per-substitution rejection of graph,
    // page, link, provider, and commit keys (see
    // `every_bounded_model_dependency_kind_matches_production_acceptance`
    // in `tests/publication_models_differential.rs`) by proving the same
    // completeness requirement at the plan-parsing boundary.
    let plan = br#"{"schema":"buoy/trace-publication-plan/v1","publication_id":"demo","selector":{"requirement_id":"DEMO-001"},"expected_rows":1,"source_set":[],"outputs":[{"id":"trace-page","mount":"repository","path":"docs/trace/{atom_revision_path}.md","kind":"page","lifecycle":"immutable"}]}"#;
    assert_eq!(
        parse_plan(plan),
        Err(PublicationError::IncompletePublicationKey)
    );
}

#[test]
fn immutable_page_path_with_a_constant_literal_only_fails_the_plan_boundary() {
    let plan = br#"{"schema":"buoy/trace-publication-plan/v1","publication_id":"demo","selector":{"requirement_id":"DEMO-001"},"expected_rows":1,"source_set":[],"outputs":[{"id":"trace-page","mount":"repository","path":"docs/trace/current.md","kind":"page","lifecycle":"immutable"}]}"#;
    assert_eq!(
        parse_plan(plan),
        Err(PublicationError::IncompletePublicationKey)
    );
}

// ---------------------------------------------------------------------------
// Round-4 direct controls: every plan-parse diagnostic must be reachable and
// must report its own stable code. A diagnostic with no direct control is a
// diagnostic nobody has ever observed.
// ---------------------------------------------------------------------------

/// Builds one plan document from an outputs fragment and a source-set
/// fragment, so each control below varies exactly one thing.
fn plan_document(source_set: &str, outputs: &str) -> String {
    format!(
        r#"{{"schema":"buoy/trace-publication-plan/v1","publication_id":"controls","selector":{{"requirement_id":"DEMO-001"}},"expected_rows":1,"source_set":[{source_set}],"outputs":[{outputs}]}}"#
    )
}

/// The stable code one plan document rejects with.
fn rejection_code(document: &str) -> String {
    parse_plan(document.as_bytes())
        .expect_err("the plan must be rejected")
        .stable_code()
        .to_owned()
}

/// An output ID is the only stable handle a report can name. An empty ID
/// silently disables report rendering for that output (`render_report`
/// rejects an empty `artifact`), turning every one of its diagnostics into a
/// bare exit code, so it must be rejected at plan parsing.
#[test]
fn an_empty_output_id_is_rejected_at_plan_parsing() {
    let document = plan_document(
        r#""openspec/specs/demo/spec.md""#,
        r#"{"id":"","mount":"repository","path":"out/page.md","kind":"page","lifecycle":"replaceable"}"#,
    );
    assert_eq!(rejection_code(&document), "TPT-PLAN-OUTPUT-ID");
}

/// An empty source-set entry can never satisfy the exact source-set equality
/// against any real selection, so it is dead weight masking a malformed plan.
#[test]
fn an_empty_source_set_entry_is_rejected_at_plan_parsing() {
    let document = plan_document(
        r#""openspec/specs/demo/spec.md","""#,
        r#"{"id":"page","mount":"repository","path":"out/page.md","kind":"page","lifecycle":"replaceable"}"#,
    );
    assert_eq!(rejection_code(&document), "TPT-SOURCE-SET-ENTRY");
}

/// A real selector never resolves one logical path twice, so a duplicated
/// declared entry can never be satisfied either.
#[test]
fn a_duplicated_source_set_entry_is_rejected_at_plan_parsing() {
    let document = plan_document(
        r#""openspec/specs/demo/spec.md","openspec/specs/demo/spec.md""#,
        r#"{"id":"page","mount":"repository","path":"out/page.md","kind":"page","lifecycle":"replaceable"}"#,
    );
    assert_eq!(rejection_code(&document), "TPT-SOURCE-SET-DUPLICATE");
}

/// A page that at least one external link references must declare the
/// `target_base` that link joins into its rendered target, at plan-parse time
/// rather than at render time.
#[test]
fn a_referenced_page_without_a_target_base_is_rejected_at_plan_parsing() {
    let document = plan_document(
        r#""openspec/specs/demo/spec.md""#,
        r#"{"id":"page","mount":"repository","path":"out/page.md","kind":"page","lifecycle":"replaceable"},{"id":"link","mount":"repository","path":"openspec/specs/demo/spec.md","kind":"external-link","lifecycle":"replaceable","relation":"documents","page":"page"}"#,
    );
    assert_eq!(rejection_code(&document), "TPT-TARGET-BASE-MISSING");
}

/// `serde_json`'s object representation silently keeps only the LAST
/// occurrence of a duplicate key, at every nesting level. An untrusted plan
/// must never have a conflicting declared value resolved that way.
#[test]
fn a_duplicate_json_key_is_rejected_before_typed_parsing() {
    // Duplicated at the top level.
    let top_level = r#"{"schema":"buoy/trace-publication-plan/v1","publication_id":"a","publication_id":"b","selector":{"requirement_id":"DEMO-001"},"expected_rows":1,"source_set":["openspec/specs/demo/spec.md"],"outputs":[{"id":"page","mount":"repository","path":"out/page.md","kind":"page","lifecycle":"replaceable"}]}"#;
    assert_eq!(rejection_code(top_level), "TPT-PLAN-DUPLICATE-KEY");
    // Duplicated inside a NESTED object, which a top-level-only guard would
    // miss entirely.
    let nested = plan_document(
        r#""openspec/specs/demo/spec.md""#,
        r#"{"id":"page","id":"other","mount":"repository","path":"out/page.md","kind":"page","lifecycle":"replaceable"}"#,
    );
    assert_eq!(rejection_code(&nested), "TPT-PLAN-DUPLICATE-KEY");
}

/// A target base is joined with a rendered page path to produce the exact
/// immutable target an external link publishes, so every non-normal form must
/// fail closed. Each case below changes which origin the joined URI actually
/// resolves against, or collapses to a different path than it appears to
/// name.
#[test]
fn every_non_normal_target_base_form_is_rejected() {
    for (base, why) in [
        ("https://user@example.invalid/trace/", "userinfo"),
        (
            "https://user:secret@example.invalid/trace/",
            "userinfo with a credential",
        ),
        ("https://example.invalid//trace/", "doubled slash"),
        ("https://example.invalid/a//b/", "interior doubled slash"),
        ("https://example.invalid/../trace/", "literal traversal"),
        ("https://example.invalid/./trace/", "literal dot segment"),
        (
            "https://example.invalid/%2e%2e/trace/",
            "percent-encoded traversal",
        ),
        (
            "https://example.invalid/%2E%2E/trace/",
            "uppercase percent-encoded traversal",
        ),
        (
            "https://example.invalid/.%2e/trace/",
            "mixed percent-encoded traversal",
        ),
        ("https://example.invalid/%2e/trace/", "encoded dot segment"),
        ("https://example.invalid:/trace/", "empty port"),
        ("https://example.invalid:port/trace/", "non-numeric port"),
        ("https://:8443/trace/", "empty host"),
        ("https://example.invalid\\trace/", "backslash separator"),
        ("http://example.invalid/trace/", "non-https scheme"),
        ("https://example.invalid/trace", "no trailing slash"),
        ("https://example.invalid/trace/?q=1", "query"),
        ("https://example.invalid/trace/#frag", "fragment"),
        ("https:///trace/", "empty authority"),
    ] {
        let document = plan_document(
            r#""openspec/specs/demo/spec.md""#,
            &format!(
                r#"{{"id":"page","mount":"repository","path":"out/page.md","kind":"page","lifecycle":"replaceable","target_base":"{base}"}},{{"id":"link","mount":"repository","path":"openspec/specs/demo/spec.md","kind":"external-link","lifecycle":"replaceable","relation":"documents","page":"page"}}"#
            ),
        );
        assert_eq!(
            rejection_code(&document),
            "TPT-TARGET-BASE",
            "a target base with {why} must be rejected: {base}"
        );
    }
}

/// The rejection above must not be over-broad: every genuinely normalized
/// base still parses. Without this control a validator that rejected
/// everything would pass the negative cases vacuously.
#[test]
fn normalized_target_bases_still_parse() {
    for base in [
        "https://example.invalid/",
        "https://example.invalid/trace/",
        "https://example.invalid:8443/trace/",
        "https://example.invalid/a/b/c/",
        "https://example.invalid/trace-v1/",
        "https://example.invalid/trace.v1/",
        "https://sub.domain.example.invalid/deep/nested/path/",
    ] {
        let document = plan_document(
            r#""openspec/specs/demo/spec.md""#,
            &format!(
                r#"{{"id":"page","mount":"repository","path":"out/page.md","kind":"page","lifecycle":"replaceable","target_base":"{base}"}},{{"id":"link","mount":"repository","path":"openspec/specs/demo/spec.md","kind":"external-link","lifecycle":"replaceable","relation":"documents","page":"page"}}"#
            ),
        );
        assert!(
            parse_plan(document.as_bytes()).is_ok(),
            "a normalized target base must still parse: {base}"
        );
    }
}
