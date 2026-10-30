//! Total adapter, determinism, editor, and execution contracts.

#![allow(clippy::expect_used, clippy::panic, clippy::too_many_lines)]

use buoy::core::{SemanticDigest, Verdict};
use chelis_provenance::command::{EXIT_EXECUTION, EXIT_OK, EXIT_STATIC, static_check};
use chelis_provenance::editor::{lint_saved_snapshot, lsp_saved_snapshot};
use chelis_provenance::execution::execute;
use chelis_provenance::records::{ChelisDialect, extract};
use chelis_provenance::snapshot::{ChelisConfiguration, ChelisSnapshot, SnapshotError};
use std::fmt::Write as _;
use std::path::Path;

const CONFIG: &[u8] =
    b"schema = \"chelis-adapter-config/v1\"\n\n[[selector]]\nroot = \"chelis\"\ndialect = \"surf\"\n\n[[selector]]\nroot = \"deep\"\ndialect = \"deep\"\n";

fn configuration() -> ChelisConfiguration {
    ChelisConfiguration::parse(CONFIG).expect("configuration")
}

fn declaration_item_digest(declaration: &str) -> String {
    buoy::core::semantic_digest(
        "chelis-raw-artifact",
        vec![(
            1,
            buoy::core::CanonicalValue::Bytes(declaration.as_bytes().to_vec()),
        )],
    )
    .expect("item digest")
    .canonical_text()
}

fn authority_source() -> String {
    String::from(
        "-- ordinary comment\n-- chelis:provenance/v1 authority\n-- id = CHELIS-EVAL-DEFAULT\n-- kind = behavioral\n-- scopes = chelis-core\n-- statement = The compiler MUST select the eval backend by default.\ndef backend(x: f32) -> f32 = x\n\ntype Option[a] = x\n",
    )
}

#[test]
fn extraction_is_total_over_selected_bytes() {
    let source = authority_source();
    let extraction = extract(ChelisDialect::Surf, source.as_bytes()).expect("extraction");
    let mut covered = vec![false; source.len()];
    let mut mark = |start: u64, end: u64| {
        let start = usize::try_from(start).expect("span start");
        let end = usize::try_from(end).expect("span end");
        for flag in covered.iter_mut().take(end).skip(start) {
            *flag = true;
        }
    };
    for record in &extraction.records {
        mark(record.start, record.end);
        mark(record.declaration_start, record.declaration_end);
    }
    for (start, end) in &extraction.comment_spans {
        mark(*start, *end);
    }
    for (start, end) in &extraction.code_spans {
        mark(*start, *end);
    }
    for diagnostic in &extraction.diagnostics {
        mark(diagnostic.start, diagnostic.end);
    }
    assert!(
        covered.into_iter().all(|flag| flag),
        "every selected byte receives exactly one total treatment"
    );
    assert_eq!(extraction.records.len(), 1);
}

#[test]
fn unsupported_schema_version_fails_loudly_without_empty_success() {
    let source = "-- chelis:provenance/v2 authority\n-- id = CHELIS-EVAL-DEFAULT\ndef f(x: f32) -> f32 = x\n";
    let error = extract(ChelisDialect::Surf, source.as_bytes()).expect_err("loud failure");
    assert_eq!(error.code(), "CHELIS-PROV-UNSUPPORTED-SCHEMA");
    let snapshot = ChelisSnapshot::from_files(vec![(
        String::from("chelis/source.ch"),
        ChelisDialect::Surf,
        source.as_bytes().to_vec(),
    )])
    .expect("snapshot");
    let outcome = static_check(&snapshot, &configuration()).expect("static outcome");
    assert_eq!(outcome.exit, EXIT_STATIC);
    assert!(
        outcome
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.code == "CHELIS-PROV-UNSUPPORTED-SCHEMA"),
        "{:?}",
        outcome.diagnostics
    );
    assert!(outcome.graph.atoms.is_empty());
    let nested = "-- chelis:provenance/v1 authority\n-- id = CHELIS-EVAL-DEFAULT\n-- kind = behavioral\n-- scopes = chelis-core\n-- statement = The subject MUST retain its saved value.\ndef nested(x: f32) -> f32 = {\n  -- chelis:provenance/v2 authority\n  y = x\n  y\n}\n";
    let snapshot = ChelisSnapshot::from_files(vec![(
        String::from("chelis/nested.ch"),
        ChelisDialect::Surf,
        nested.as_bytes().to_vec(),
    )])
    .expect("nested snapshot");
    let outcome = static_check(&snapshot, &configuration()).expect("nested outcome");
    assert_eq!(outcome.exit, EXIT_STATIC);
    assert!(outcome.graph.atoms.is_empty());
    let diagnostic = outcome
        .diagnostics
        .iter()
        .find(|diagnostic| diagnostic.code == "CHELIS-PROV-UNSUPPORTED-SCHEMA")
        .expect("nested schema defect");
    let start = nested
        .find("  -- chelis:provenance/v2")
        .expect("nested marker") as u64;
    assert_eq!(diagnostic.start, start);
    assert_eq!(
        diagnostic.end,
        start + "  -- chelis:provenance/v2 authority\n".len() as u64
    );
    assert_eq!(
        diagnostic.context,
        "unsupported-marker:chelis:provenance/v2 authority"
    );
}

#[test]
fn malformed_and_unattached_records_produce_stable_diagnostics() {
    let malformed =
        "-- chelis:provenance/v1 authority\n-- id CHELIS-EVAL-DEFAULT\ndef f(x: f32) -> f32 = x\n";
    let extraction = extract(ChelisDialect::Surf, malformed.as_bytes()).expect("extraction");
    assert_eq!(extraction.diagnostics.len(), 1);
    assert_eq!(
        extraction.diagnostics[0].code,
        "CHELIS-PROV-MALFORMED-RECORD"
    );

    for fields in [
        "-- = value\n",
        "-- bad line\n-- id = X\n-- id = X\n-- another bad line\n",
        "-- id = X\n-- id = X\n-- bad line\n",
    ] {
        let mut text = String::from("-- chelis:provenance/v1 authority\n");
        text.push_str(fields);
        text.push_str("def subject(x: f32) -> f32 = x\n");
        let extraction = extract(ChelisDialect::Surf, text.as_bytes()).expect("record grammar");
        assert!(extraction.records.is_empty());
        assert_eq!(extraction.diagnostics.len(), 1);
        assert_eq!(
            extraction.diagnostics[0].code,
            "CHELIS-PROV-MALFORMED-RECORD"
        );
        assert_eq!(
            extraction.diagnostics[0].context,
            if fields.starts_with("-- id") {
                "duplicate-field:id"
            } else {
                "invalid-field-line"
            }
        );
    }
    let digit_key = authority_source().replace("-- kind =", "-- key1 = accepted\n-- kind =");
    let extraction = extract(ChelisDialect::Surf, digit_key.as_bytes()).expect("digit key");
    assert!(extraction.diagnostics.is_empty());
    assert_eq!(extraction.records[0].fields["key1"], "accepted");
    for suffix in ["", "module Demo.Mod\n"] {
        let mut text = String::from("-- chelis:provenance/v1 authority\n-- id = X\n");
        text.push_str(suffix);
        let extraction = extract(ChelisDialect::Surf, text.as_bytes()).expect("unattached record");
        assert!(extraction.records.is_empty());
        assert_eq!(
            extraction.diagnostics[0].code,
            "CHELIS-PROV-UNATTACHED-METADATA"
        );
    }
    let consecutive = "-- chelis:provenance/v1 authority\n-- chelis:provenance/v1 authority\n-- id = X\ndef subject(x: f32) -> f32 = x\n";
    let extraction =
        extract(ChelisDialect::Surf, consecutive.as_bytes()).expect("consecutive markers");
    assert_eq!(extraction.records.len(), 1);
    assert_eq!(extraction.records[0].fields["id"], "X");
    assert_eq!(
        extraction.diagnostics[0].code,
        "CHELIS-PROV-UNATTACHED-METADATA"
    );

    let unattached = "-- chelis:provenance/v1 authority\n-- id = CHELIS-EVAL-DEFAULT\n\n-- trailing comment only\n";
    let extraction = extract(ChelisDialect::Surf, unattached.as_bytes()).expect("extraction");
    assert_eq!(extraction.diagnostics.len(), 1);
    assert_eq!(
        extraction.diagnostics[0].code,
        "CHELIS-PROV-UNATTACHED-METADATA"
    );

    let unknown_kind =
        "-- chelis:provenance/v1 mystery\n-- id = CHELIS-EVAL-DEFAULT\ndef f(x: f32) -> f32 = x\n";
    let snapshot = ChelisSnapshot::from_files(vec![(
        String::from("chelis/source.ch"),
        ChelisDialect::Surf,
        unknown_kind.as_bytes().to_vec(),
    )])
    .expect("snapshot");
    let outcome = static_check(&snapshot, &configuration()).expect("static outcome");
    assert!(
        outcome.diagnostics.iter().any(|diagnostic| diagnostic.code
            == "CHELIS-PROV-MALFORMED-RECORD"
            && diagnostic.context == "unknown-record-kind:mystery"),
        "{:?}",
        outcome.diagnostics
    );

    for (dialect, bytes, context) in [
        (ChelisDialect::Surf, vec![0xff, 0xfe], "invalid-utf8"),
        (
            ChelisDialect::Surf,
            b"-- chelis:provenance/v1\n".to_vec(),
            "missing-record-kind",
        ),
        (
            ChelisDialect::Deep,
            b"// chelis:provenance/v1 authority\n// Invalid = value\n(def subject x)\n".to_vec(),
            "invalid-field-line",
        ),
    ] {
        let snapshot = ChelisSnapshot::from_files(vec![(
            String::from("chelis/invalid"),
            dialect,
            bytes.clone(),
        )])
        .expect("snapshot");
        let outcome = static_check(&snapshot, &configuration()).expect("static outcome");
        assert_eq!(outcome.exit, EXIT_STATIC);
        assert!(outcome.graph.atoms.is_empty());
        let diagnostic = outcome
            .diagnostics
            .iter()
            .find(|diagnostic| {
                diagnostic.code == "CHELIS-PROV-MALFORMED-RECORD" && diagnostic.context == context
            })
            .expect("input defect");
        let record_end = bytes
            .iter()
            .position(|byte| *byte == b'(')
            .unwrap_or(bytes.len());
        assert_eq!((diagnostic.start, diagnostic.end), (0, record_end as u64));
    }

    let deep = "// chelis:provenance/v1 authority\n// id = CHELIS-EVAL-DEFAULT\n";
    let extraction = extract(ChelisDialect::Deep, deep.as_bytes()).expect("extraction");
    assert!(extraction.records.is_empty());
    assert_eq!(
        extraction.diagnostics[0].code,
        "CHELIS-PROV-UNATTACHED-METADATA"
    );
    assert_eq!(extraction.diagnostics[0].context, "record-kind:authority");
}

/// The compact form's whole claim is that no in-source field can drift, which
/// holds only while its field set is closed. These are grammar facts, so they
/// hold under every digest member and run in the default build.
#[test]
fn the_binding_marker_carries_exactly_one_address_field() {
    let address = format!("blake3-256:{}", "33".repeat(32));
    let declaration = "def bound(x: f32) -> f32 = x\n";

    let valid = format!("-- chelis:provenance/v1 binding\n-- record = {address}\n{declaration}");
    let extraction = extract(ChelisDialect::Surf, valid.as_bytes()).expect("extraction");
    assert!(extraction.diagnostics.is_empty(), "{extraction:?}");
    assert_eq!(extraction.records.len(), 1);
    let record = &extraction.records[0];
    assert_eq!(record.kind, "binding");
    assert_eq!(
        record.fields.keys().collect::<Vec<_>>(),
        vec!["record"],
        "the marker carries exactly the address field"
    );
    assert_eq!(record.fields["record"], address);
    assert_eq!(
        record.declaration_text,
        declaration.trim_end(),
        "the marker anchors to the next declaration"
    );

    let unattached = format!(
        "-- chelis:provenance/v1 binding\n-- record = {address}\n\n-- trailing comment only\n"
    );
    let extraction = extract(ChelisDialect::Surf, unattached.as_bytes()).expect("extraction");
    assert!(extraction.records.is_empty());
    assert_eq!(extraction.diagnostics.len(), 1);
    assert_eq!(
        extraction.diagnostics[0].code,
        "CHELIS-PROV-UNATTACHED-METADATA"
    );
    assert_eq!(extraction.diagnostics[0].context, "record-kind:binding");

    // Duplicates keep the reason extraction already gives every record, so the
    // closed field set never masks a more precise defect.
    let duplicate = format!(
        "-- chelis:provenance/v1 binding\n-- record = {address}\n-- record = {address}\n{declaration}"
    );
    let extraction = extract(ChelisDialect::Surf, duplicate.as_bytes()).expect("extraction");
    assert!(extraction.records.is_empty());
    assert_eq!(extraction.diagnostics.len(), 1);
    assert_eq!(
        extraction.diagnostics[0].code,
        "CHELIS-PROV-MALFORMED-RECORD"
    );
    assert_eq!(extraction.diagnostics[0].context, "duplicate-field:record");

    for (extra, context) in [
        (
            format!("-- record = {address}\n-- navigation = implements\n"),
            "unknown-field:navigation",
        ),
        (String::from("-- id = SOME-LINK\n"), "unknown-field:id"),
        (String::new(), "missing-field:record"),
    ] {
        let source = format!("-- chelis:provenance/v1 binding\n{extra}{declaration}");
        let extraction = extract(ChelisDialect::Surf, source.as_bytes()).expect("extraction");
        assert!(extraction.records.is_empty(), "{context}: {extraction:?}");
        assert_eq!(extraction.diagnostics.len(), 1, "{context}");
        assert_eq!(
            extraction.diagnostics[0].code,
            "CHELIS-PROV-MALFORMED-RECORD"
        );
        assert_eq!(extraction.diagnostics[0].context, context);
    }
}

/// A syntactically valid marker whose address is not a digest is a grammar
/// success and a resolution failure, in every build.
#[test]
fn a_malformed_binding_address_is_rejected_at_resolution() {
    let source =
        "-- chelis:provenance/v1 binding\n-- record = not-a-digest\ndef bound(x: f32) -> f32 = x\n";
    let extraction = extract(ChelisDialect::Surf, source.as_bytes()).expect("extraction");
    assert!(extraction.diagnostics.is_empty(), "{extraction:?}");
    assert_eq!(extraction.records.len(), 1);

    let snapshot = ChelisSnapshot::from_files(vec![(
        String::from("chelis/source.ch"),
        ChelisDialect::Surf,
        source.as_bytes().to_vec(),
    )])
    .expect("snapshot");
    let outcome = static_check(&snapshot, &configuration()).expect("static outcome");
    assert_eq!(outcome.exit, EXIT_STATIC);
    assert!(
        outcome.diagnostics.iter().any(|diagnostic| diagnostic.code
            == "CHELIS-PROV-BINDING-ADDRESS"
            && diagnostic.context == "invalid-address"),
        "{:?}",
        outcome.diagnostics
    );
    assert!(outcome.graph.implementation_bindings.is_empty());
}

#[test]
fn parser_identity_survives_reformatting_around_the_declaration() {
    let record = "-- chelis:provenance/v1 authority\n-- id = CHELIS-EVAL-DEFAULT\n-- kind = behavioral\n-- scopes = chelis-core\n-- statement = The compiler MUST select the eval backend by default.\n";
    let declaration = "def backend(x: f32) -> f32 = x\n";
    let before = format!("{record}{declaration}");
    let after = format!("-- moved preamble comment\n\n\n{record}{declaration}");
    let first = extract(ChelisDialect::Surf, before.as_bytes()).expect("first extraction");
    let second = extract(ChelisDialect::Surf, after.as_bytes()).expect("second extraction");
    assert_eq!(first.records.len(), 1);
    assert_eq!(second.records.len(), 1);
    assert_eq!(
        first.records[0].declaration_text, second.records[0].declaration_text,
        "identity bytes derive from the declaration node, not its location"
    );
    assert_ne!(
        first.records[0].declaration_start, second.records[0].declaration_start,
        "the span tracks the new location"
    );
}

#[test]
fn multi_line_declaration_binds_whole_node_bytes() {
    let source = "-- chelis:provenance/v1 authority\n-- id = CHELIS-EVAL-DEFAULT\n-- kind = behavioral\n-- scopes = chelis-core\n-- statement = The compiler MUST select the eval backend by default.\ndef backend(x: f32) -> f32 = {\n  y = x\n  y\n}\n\ndef untouched(x: f32) -> f32 = x\n";
    let extraction = extract(ChelisDialect::Surf, source.as_bytes()).expect("extraction");
    assert_eq!(extraction.records.len(), 1);
    let text = &extraction.records[0].declaration_text;
    assert!(
        text.starts_with("def backend"),
        "node starts at def: {text}"
    );
    assert!(
        text.ends_with('}'),
        "node ends at the closing brace: {text}"
    );
    assert!(
        text.contains("y = x"),
        "body bytes participate in identity: {text}"
    );
    let final_source = source
        .replace("\n  y = x", "\n  -- note\n  y = x")
        .replace("\n\ndef untouched(x: f32) -> f32 = x\n", "");
    let final_source = final_source.trim_end_matches('\n');
    let final_record =
        extract(ChelisDialect::Surf, final_source.as_bytes()).expect("final declaration");
    assert!(final_record.diagnostics.is_empty());
    assert_eq!(
        final_record.records[0].declaration_text,
        final_source[final_source.find("def backend").expect("declaration")..]
    );
    assert!(final_record.records[0].declaration_text.contains("-- note"));
}

#[test]
fn parse_error_fails_loudly_without_empty_success() {
    let source =
        "-- chelis:provenance/v1 authority\n-- id = CHELIS-EVAL-DEFAULT\ndef broken( = {\n";
    let error = extract(ChelisDialect::Surf, source.as_bytes()).expect_err("loud parse failure");
    assert_eq!(error.code(), "CHELIS-PROV-PARSE-ERROR");
    let snapshot = ChelisSnapshot::from_files(vec![(
        String::from("chelis/source.ch"),
        ChelisDialect::Surf,
        source.as_bytes().to_vec(),
    )])
    .expect("snapshot");
    let outcome = static_check(&snapshot, &configuration()).expect("static outcome");
    assert_eq!(outcome.exit, EXIT_STATIC);
    assert!(
        outcome
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.code == "CHELIS-PROV-PARSE-ERROR"),
        "{:?}",
        outcome.diagnostics
    );
    assert!(outcome.graph.atoms.is_empty());
}

#[test]
fn record_inside_declaration_body_is_unattached() {
    let source = "-- chelis:provenance/v1 authority\n-- id = CHELIS-EVAL-DEFAULT\n-- kind = behavioral\n-- scopes = chelis-core\n-- statement = The compiler MUST select the eval backend by default.\ndef backend(x: f32) -> f32 = {\n  -- chelis:provenance/v1 authority\n  y = x\n  y\n}\n";
    let extraction = extract(ChelisDialect::Surf, source.as_bytes()).expect("extraction");
    assert_eq!(extraction.records.len(), 1, "the outer record still binds");
    assert!(
        extraction
            .diagnostics
            .iter()
            .any(
                |diagnostic| diagnostic.code == "CHELIS-PROV-UNATTACHED-METADATA"
                    && diagnostic.context == "inside-declaration"
            ),
        "a marker inside a declaration body stays visible: {:?}",
        extraction.diagnostics
    );
}

#[test]
fn stale_item_digest_fails_with_bound_and_current_context() {
    let true_digest = declaration_item_digest("def eval_impl(x: f32) -> f32 = x");
    let stale_digest = "xxh3-128:000000000000000000000000000000dd";
    let atom = buoy::core::Atom::new(
        buoy::core::StableId::new("CHELIS-EVAL-DEFAULT").expect("atom id"),
        buoy::core::LifecycleState::Proposed,
        buoy::core::AtomKind::Behavioral,
        vec![buoy::core::ScopeId::new("chelis-core").expect("scope")],
        "The compiler MUST select the eval backend by default.",
        None,
    )
    .expect("atom value");
    let revision = atom.revision().canonical_text();
    let source = format!(
        "-- chelis:provenance/v1 authority\n-- id = CHELIS-EVAL-DEFAULT\n-- kind = behavioral\n-- scopes = chelis-core\n-- statement = The compiler MUST select the eval backend by default.\ndef backend(x: f32) -> f32 = x\n\
         -- chelis:provenance/v1 implementation-link\n-- id = CHELIS-EVAL-LINK\n-- atoms = CHELIS-EVAL-DEFAULT@{revision}\n-- item-digest = {stale_digest}\n-- navigation = implements\ndef eval_impl(x: f32) -> f32 = x\n"
    );
    let snapshot = ChelisSnapshot::from_files(vec![(
        String::from("chelis/source.ch"),
        ChelisDialect::Surf,
        source.into_bytes(),
    )])
    .expect("snapshot");
    let outcome = static_check(&snapshot, &configuration()).expect("static outcome");
    assert_eq!(outcome.exit, EXIT_STATIC);
    let diagnostic = outcome
        .diagnostics
        .iter()
        .find(|diagnostic| diagnostic.code == "CHELIS-PROV-ITEM-STALE")
        .expect("item staleness must be enforced");
    assert_eq!(
        diagnostic.context,
        format!("link:CHELIS-EVAL-LINK;bound={stale_digest};current={true_digest}"),
        "context carries bound and current values"
    );
}

#[test]
fn oracle_runner_binding_is_enforced_against_saved_artifacts() {
    let reef = b"version = \"1.0.0\"\n".to_vec();
    let reef_digest = chelis_provenance::command::artifact_digest(&reef)
        .expect("artifact digest")
        .canonical_text();
    let stale = "xxh3-128:000000000000000000000000000000ee";
    let oracle = |runner_digest: &str| {
        format!(
            "-- chelis:provenance/v1 oracle\n-- id = CHELIS-RUNNER-ORACLE\n-- oracle-kind = test\n-- runner-id = chelis-test\n-- runner-version = 1.0.0\n-- runner-digest = {runner_digest}\n-- runner-configuration-digest = {runner_digest}\n-- runner-config-paths = chelis/reef.toml\n-- selection-schema = chelis-oracle-selection/v1\n-- selection = selection\n-- normalization-schema = chelis-oracle-normalization/v1\n-- normalization = normalization\n-- configuration-digest = xxh3-128:00000000000000000000000000000003\n-- scope-schema = chelis-oracle-scope/v1\n-- scope = scope\ndef oracle_marker(x: f32) -> f32 = x\n"
        )
    };
    let build = |runner_digest: &str| {
        ChelisSnapshot::from_files_with_artifacts(
            vec![(
                String::from("chelis/source.ch"),
                ChelisDialect::Surf,
                oracle(runner_digest).into_bytes(),
            )],
            vec![(String::from("chelis/reef.toml"), reef.clone())],
        )
        .expect("snapshot")
    };
    let current = static_check(&build(&reef_digest), &configuration()).expect("current outcome");
    assert!(
        !current
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.code == "CHELIS-PROV-ORACLE-RUNNER-STALE"),
        "{:?}",
        current.diagnostics
    );
    let outcome = static_check(&build(stale), &configuration()).expect("stale outcome");
    assert_eq!(outcome.exit, EXIT_STATIC);
    let diagnostics = outcome
        .diagnostics
        .iter()
        .filter(|diagnostic| diagnostic.code == "CHELIS-PROV-ORACLE-RUNNER-STALE")
        .collect::<Vec<_>>();
    assert_eq!(diagnostics.len(), 1, "{:?}", outcome.diagnostics);
    assert_eq!(
        diagnostics[0].context,
        format!(
            "oracle:CHELIS-RUNNER-ORACLE:runner-digest:chelis/reef.toml;bound={stale};current={reef_digest}"
        )
    );
    for paths in [";", "a;b;c"] {
        let text = oracle(&reef_digest).replace("chelis/reef.toml", paths);
        let saved = ChelisSnapshot::from_files(vec![(
            String::from("chelis/source.ch"),
            ChelisDialect::Surf,
            text.into_bytes(),
        )])
        .expect("invalid runner paths");
        let outcome = static_check(&saved, &configuration()).expect("runner paths outcome");
        assert_eq!(outcome.exit, EXIT_STATIC);
        assert!(
            outcome
                .diagnostics
                .iter()
                .any(|row| row.code == "CHELIS-PROV-MALFORMED-RECORD"
                    && row.context == "invalid-runner-config-paths:CHELIS-RUNNER-ORACLE"),
            "{:?}",
            outcome.diagnostics
        );
    }
}

#[test]
fn revision_staleness_context_names_bound_and_current() {
    let atom = buoy::core::Atom::new(
        buoy::core::StableId::new("CHELIS-EVAL-DEFAULT").expect("atom id"),
        buoy::core::LifecycleState::Proposed,
        buoy::core::AtomKind::Behavioral,
        vec![buoy::core::ScopeId::new("chelis-core").expect("scope")],
        "The compiler MUST select the eval backend by default.",
        None,
    )
    .expect("atom value");
    let stale_revision = buoy::core::Atom::new(
        buoy::core::StableId::new("CHELIS-EVAL-DEFAULT").expect("atom id"),
        buoy::core::LifecycleState::Proposed,
        buoy::core::AtomKind::Behavioral,
        vec![buoy::core::ScopeId::new("chelis-core").expect("scope")],
        "The compiler MUST select the prior eval backend.",
        None,
    )
    .expect("stale atom value")
    .revision()
    .canonical_text();
    let revision = atom.revision().canonical_text();
    let item = declaration_item_digest("def eval_impl(x: f32) -> f32 = x");
    let source = format!(
        "-- chelis:provenance/v1 authority\n-- id = CHELIS-EVAL-DEFAULT\n-- kind = behavioral\n-- scopes = chelis-core\n-- statement = The compiler MUST select the eval backend by default.\ndef backend(x: f32) -> f32 = x\n\
         -- chelis:provenance/v1 implementation-link\n-- id = CHELIS-EVAL-LINK\n-- atoms = CHELIS-EVAL-DEFAULT@{stale_revision}\n-- item-digest = {item}\n-- navigation = implements\ndef eval_impl(x: f32) -> f32 = x\n"
    );
    let snapshot = ChelisSnapshot::from_files(vec![(
        String::from("chelis/source.ch"),
        ChelisDialect::Surf,
        source.as_bytes().to_vec(),
    )])
    .expect("snapshot");
    let outcome = static_check(&snapshot, &configuration()).expect("static outcome");
    let diagnostic = outcome
        .diagnostics
        .iter()
        .find(|diagnostic| diagnostic.code == "SPG-REF-DANGLING-OR-STALE")
        .expect("revision staleness must be visible");
    assert_eq!(
        diagnostic.context,
        format!(
            "CHELIS-EVAL-LINK;atom=CHELIS-EVAL-DEFAULT;bound={stale_revision};current={revision}"
        ),
        "context carries bound and current revisions"
    );
    let property = source.replace("-- chelis:provenance/v1 implementation-link", "-- chelis:provenance/v1 formal-property")
        .replace("-- navigation = implements", "-- property-schema = property/v1\n-- property = Identity function\n-- abstraction-schema = abstraction/v1\n-- abstraction = Pure identity\n-- approval-authority = chelis-core\n-- approval-record = reviews/property.md\n-- approval-digest = xxh3-128:00000000000000000000000000000001");
    let saved = ChelisSnapshot::from_files(vec![(
        String::from("chelis/source.ch"),
        ChelisDialect::Surf,
        property.into_bytes(),
    )])
    .expect("stale formal property");
    let outcome = static_check(&saved, &configuration()).expect("formal property outcome");
    let error = outcome
        .diagnostics
        .iter()
        .find(|row| row.code == "SPG-REF-DANGLING-OR-STALE")
        .expect("formal property stale reference");
    assert_eq!(
        error.context,
        format!(
            "CHELIS-EVAL-LINK;atom=CHELIS-EVAL-DEFAULT;bound={stale_revision};current={revision}"
        )
    );
}

#[test]
fn deep_dialect_extracts_records_with_deep_item_kind() {
    let source = "// chelis:provenance/v1 authority\n// id = CHELIS-DEEP-META\n// kind = structural\n// scopes = chelis-core\n// statement = Deep metadata MUST stay attached to its node.\n(def {} square (fn {} (params {} x) x))\n";
    let snapshot = ChelisSnapshot::from_files(vec![(
        String::from("deep/source.dp"),
        ChelisDialect::Deep,
        source.as_bytes().to_vec(),
    )])
    .expect("snapshot");
    let outcome = static_check(&snapshot, &configuration()).expect("static outcome");
    assert_eq!(outcome.graph.atoms.len(), 1);
    assert_eq!(outcome.exit, EXIT_OK, "{:?}", outcome.diagnostics);
}

#[test]
fn static_reports_are_root_and_order_independent() {
    let surf = authority_source();
    let deep = "// chelis:provenance/v1 authority\n// id = CHELIS-DEEP-META\n// kind = structural\n// scopes = chelis-core\n// statement = Deep metadata MUST stay attached to its node.\n(def {} square x)\n";
    let forward = ChelisSnapshot::from_files(vec![
        (
            String::from("chelis/a.ch"),
            ChelisDialect::Surf,
            surf.as_bytes().to_vec(),
        ),
        (
            String::from("deep/b.dp"),
            ChelisDialect::Deep,
            deep.as_bytes().to_vec(),
        ),
    ])
    .expect("forward snapshot");
    let reverse = ChelisSnapshot::from_files(vec![
        (
            String::from("deep/b.dp"),
            ChelisDialect::Deep,
            deep.as_bytes().to_vec(),
        ),
        (
            String::from("chelis/a.ch"),
            ChelisDialect::Surf,
            surf.as_bytes().to_vec(),
        ),
    ])
    .expect("reverse snapshot");
    let first = static_check(&forward, &configuration()).expect("first outcome");
    let second = static_check(&reverse, &configuration()).expect("second outcome");
    assert_eq!(
        first.report, second.report,
        "registration and input order cannot alter canonical bytes"
    );
    assert_eq!(first.identity, second.identity);

    let root_a = tempfile::tempdir().expect("root a");
    let root_b = tempfile::tempdir().expect("root b");
    for root in [&root_a, &root_b] {
        std::fs::create_dir_all(root.path().join("chelis")).expect("selector root");
        std::fs::write(root.path().join("chelis/a.ch"), surf.as_bytes()).expect("write source");
    }
    let from_a =
        ChelisSnapshot::from_directory(root_a.path(), &configuration()).expect("snapshot a");
    let from_b =
        ChelisSnapshot::from_directory(root_b.path(), &configuration()).expect("snapshot b");
    let outcome_a = static_check(&from_a, &configuration()).expect("outcome a");
    let outcome_b = static_check(&from_b, &configuration()).expect("outcome b");
    assert_eq!(
        outcome_a.report, outcome_b.report,
        "distinct absolute roots produce identical report bytes"
    );
}

#[test]
fn snapshot_construction_rejects_ambient_and_unsafe_inputs() {
    let overlap = ChelisConfiguration::parse(
        b"schema = \"chelis-adapter-config/v1\"\n\n[[selector]]\nroot = \"chelis\"\ndialect = \"surf\"\n\n[[selector]]\nroot = \"chelis/nested\"\ndialect = \"deep\"\n",
    );
    assert!(matches!(overlap, Err(SnapshotError::SelectorOverlap(_))));
    for (first, second) in [("a", "a"), ("a/b", "a"), ("", "a")] {
        let mut text = String::from("schema = \"chelis-adapter-config/v1\"\n");
        for root in [first, second] {
            writeln!(text, "[[selector]]\nroot = \"{root}\"\ndialect = \"surf\"")
                .expect("selector");
        }
        assert!(matches!(
            ChelisConfiguration::parse(text.as_bytes()),
            Err(SnapshotError::SelectorOverlap(_))
        ));
    }

    let collision = ChelisSnapshot::from_files(vec![
        (String::from("chelis/A.ch"), ChelisDialect::Surf, Vec::new()),
        (String::from("chelis/a.ch"), ChelisDialect::Surf, Vec::new()),
    ]);
    assert!(matches!(collision, Err(SnapshotError::PathCollision(_))));
    let normalized_collision = ChelisSnapshot::from_files(vec![
        (
            String::from("chelis/caf\u{e9}.ch"),
            ChelisDialect::Surf,
            Vec::new(),
        ),
        (
            String::from("chelis/cafe\u{301}.ch"),
            ChelisDialect::Surf,
            Vec::new(),
        ),
    ]);
    assert_eq!(
        normalized_collision,
        Err(SnapshotError::PathCollision(String::from(
            "chelis/cafe\u{301}.ch"
        )))
    );

    #[cfg(unix)]
    {
        let root = tempfile::tempdir().expect("root");
        std::fs::create_dir_all(root.path().join("chelis/nested")).expect("selector root");
        std::fs::write(
            root.path().join("chelis/a.ch"),
            b"def f(x: f32) -> f32 = x\n",
        )
        .expect("write source");
        std::os::unix::fs::symlink(
            root.path().join("chelis/a.ch"),
            root.path().join("chelis/nested/link.ch"),
        )
        .expect("symlink");
        let result = ChelisSnapshot::from_directory(root.path(), &configuration());
        assert!(matches!(result, Err(SnapshotError::Symlink(_))));
    }

    let root = tempfile::tempdir().expect("root");
    std::fs::create_dir_all(root.path().join("chelis/nested/.git")).expect("nested boundary");
    std::fs::write(
        root.path().join("chelis/a.ch"),
        b"def f(x: f32) -> f32 = x\n",
    )
    .expect("write source");
    let result = ChelisSnapshot::from_directory(root.path(), &configuration());
    assert!(matches!(result, Err(SnapshotError::RepositoryBoundary(_))));

    assert_eq!(
        ChelisSnapshot::from_files(vec![(
            String::from("../escape.ch"),
            ChelisDialect::Surf,
            Vec::new(),
        )]),
        Err(SnapshotError::Path(String::from("../escape.ch")))
    );
    assert_eq!(
        ChelisSnapshot::from_files_with_artifacts(
            vec![(String::from("chelis/A.ch"), ChelisDialect::Surf, Vec::new())],
            vec![(String::from("chelis/a.ch"), Vec::new())],
        ),
        Err(SnapshotError::PathCollision(String::from(
            "chelis/A.ch|chelis/a.ch"
        )))
    );
    assert_eq!(
        ChelisSnapshot::from_files_with_artifacts(
            Vec::new(),
            vec![
                (String::from("chelis/REEF.toml"), Vec::new()),
                (String::from("chelis/reef.toml"), Vec::new()),
            ],
        ),
        Err(SnapshotError::PathCollision(String::from(
            "chelis/REEF.toml|chelis/reef.toml"
        )))
    );

    let root = tempfile::tempdir().expect("root");
    std::fs::write(root.path().join("chelis"), b"not a directory").expect("selector file");
    assert!(matches!(
        ChelisSnapshot::from_directory(root.path(), &configuration()),
        Err(SnapshotError::Read(_))
    ));
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let root = tempfile::tempdir().expect("unreadable root");
        std::fs::create_dir_all(root.path().join("chelis/nested")).expect("nested directory");
        let path = root.path().join("chelis/nested/private.ch");
        std::fs::write(&path, authority_source()).expect("private source");
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o0)).expect("deny read");
        let read_denied = std::fs::read(&path).is_err();
        let outcome = ChelisSnapshot::from_directory(root.path(), &configuration());
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o600))
            .expect("restore read");
        if read_denied {
            assert!(matches!(outcome, Err(SnapshotError::Read(_))));
        }
    }
}

#[test]
fn lint_and_lsp_consume_the_same_semantic_library_over_saved_snapshots() {
    let source = "-- chelis:provenance/v1 authority\n-- id = CHELIS-EVAL-DEFAULT\ndef f(x: f32) -> f32 = x\n";
    let snapshot = ChelisSnapshot::from_files(vec![(
        String::from("chelis/source.ch"),
        ChelisDialect::Surf,
        source.as_bytes().to_vec(),
    )])
    .expect("snapshot");
    for findings in [
        lint_saved_snapshot(&snapshot, &configuration()).expect("lint findings"),
        lsp_saved_snapshot(&snapshot, &configuration()).expect("lsp findings"),
    ] {
        let defect = findings
            .iter()
            .find(|diagnostic| diagnostic.code == "CHELIS-PROV-MALFORMED-RECORD")
            .expect("missing authority fields");
        assert_eq!(defect.context, "missing-field:kind");
        assert_eq!(defect.path, "chelis/source.ch");
        assert_eq!(defect.start, 0);
        assert_eq!(defect.end, (source.len() - 1) as u64);
    }
    let repaired = ChelisSnapshot::from_files(vec![(
        String::from("chelis/source.ch"),
        ChelisDialect::Surf,
        authority_source().into_bytes(),
    )])
    .expect("repaired snapshot");
    let view = chelis_provenance::editor::editor_static_view(&repaired, &configuration())
        .expect("editor view");
    assert_eq!(view.exit, EXIT_OK);
    assert!(view.diagnostics.is_empty());
    assert_eq!(
        view.graph.atoms[0].meta().id().as_str(),
        "CHELIS-EVAL-DEFAULT"
    );
    assert!(
        lint_saved_snapshot(&repaired, &configuration())
            .expect("lint repair")
            .is_empty()
    );
    assert!(
        lsp_saved_snapshot(&repaired, &configuration())
            .expect("lsp repair")
            .is_empty()
    );
}

#[test]
fn every_remaining_record_kind_maps_into_repository_independent_values() {
    let atom = buoy::core::Atom::new(
        buoy::core::StableId::new("CHELIS-EVAL-DEFAULT").expect("atom id"),
        buoy::core::LifecycleState::Proposed,
        buoy::core::AtomKind::Behavioral,
        vec![buoy::core::ScopeId::new("chelis-core").expect("scope")],
        "The compiler MUST select the eval backend by default.",
        None,
    )
    .expect("atom value");
    let revision = atom.revision().canonical_text();
    let object_digest = atom.meta().digest().canonical_text();
    let digest = declaration_item_digest("def backend_impl(x: f32) -> f32 = x");
    let head_digest = declaration_item_digest("def backend_impl(x: f32) -> f32 = x + x");
    let source = format!(
        "-- chelis:provenance/v1 authority\n-- id = CHELIS-EVAL-DEFAULT\n-- kind = behavioral\n-- scopes = chelis-core\n-- statement = The compiler MUST select the eval backend by default.\ndef backend(x: f32) -> f32 = x\n\
         -- chelis:provenance/v1 implementation-link\n-- id = CHELIS-EVAL-LINK\n-- atoms = CHELIS-EVAL-DEFAULT@{revision}\n-- item-digest = {digest}\n-- navigation = implements\ndef backend_impl(x: f32) -> f32 = x\n\
         -- chelis:provenance/v1 surface\n-- id = CHELIS-BACKEND-SURFACE\n-- member-key-schema = chelis-backend-member/v1\n-- disposition-schema = chelis-backend-disposition/v1\n-- generation = 1\n-- members = eval;metal\n-- rows = eval:normative:CHELIS-EVAL-DEFAULT@{revision};metal:not-applicable\ndef backend_surface(x: f32) -> f32 = x\n\
         -- chelis:provenance/v1 reaffirmation\n-- id = CHELIS-EVAL-REAFFIRMATION\n-- affected-kind = evidence-carrier\n-- affected-id = CHELIS-BACKEND-CARRIER\n-- base-digest = {digest}\n-- head-digest = {head_digest}\n-- approval-authority = chelis-core\n-- approval-record = reviews/reaffirm.md\n-- approval-digest = {digest}\n-- statement = Reviewed and deliberately reaffirmed under new bindings.\ndef backend_reaffirm(x: f32) -> f32 = x\n\
         -- chelis:provenance/v1 formal-property\n-- id = CHELIS-EVAL-PROPERTY\n-- atoms = CHELIS-EVAL-DEFAULT@{revision}\n-- property-schema = chelis-formal-property/v1\n-- property = backend selection is deterministic\n-- abstraction-schema = chelis-abstraction/v1\n-- abstraction = pure selection function\n-- approval-authority = chelis-core\n-- approval-record = reviews/property.md\n-- approval-digest = {digest}\ndef backend_property(x: f32) -> f32 = x\n\
         -- chelis:provenance/v1 debt\n-- id = CHELIS-EVAL-DEBT\n-- obligations = CHELIS-BACKEND-POLICY:freshness:atom:CHELIS-EVAL-DEFAULT@{object_digest}\n-- objects = atom:CHELIS-EVAL-DEFAULT@{object_digest}\n-- rationale = visible remediation debt for a stale binding\n-- owner = chelis-core\n-- remediation = rebind the carrier to the current revision\n-- discharge-schema = chelis-discharge/v1\n-- discharge = carrier rebinds and passes\ndef backend_debt(x: f32) -> f32 = x\n"
    );
    let snapshot = ChelisSnapshot::from_files(vec![(
        String::from("chelis/kinds.ch"),
        ChelisDialect::Surf,
        source.into_bytes(),
    )])
    .expect("snapshot");
    let outcome = static_check(&snapshot, &configuration()).expect("static outcome");
    assert!(
        !outcome
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.code == "CHELIS-PROV-MALFORMED-RECORD"),
        "{:?}",
        outcome.diagnostics
    );
    assert_eq!(outcome.graph.atoms.len(), 1);
    assert_eq!(outcome.graph.implementation_links.len(), 1);
    assert_eq!(outcome.graph.decision_surfaces.len(), 1);
    assert_eq!(outcome.graph.reaffirmations.len(), 1);
    assert_eq!(outcome.graph.formal_properties.len(), 1);
    assert_eq!(outcome.graph.debt_records.len(), 1);
}

#[test]
fn execution_receipts_are_five_state_and_cannot_mutate_static_identity() {
    let identity = SemanticDigest::parse_canonical("xxh3-128:000000000000000000000000000000aa")
        .expect("identity");
    let sentinels = [
        ("O-PASS", "builtin:pass", Verdict::Pass),
        ("O-FAIL", "builtin:fail", Verdict::Fail),
        ("O-ERROR", "builtin:error", Verdict::Error),
        ("O-TIMEOUT", "builtin:timeout", Verdict::Timeout),
        ("O-NOT-RUN", "builtin:not-run", Verdict::NotRun),
    ];
    let commands = sentinels
        .iter()
        .map(|(id, program, _)| {
            toml::from_str::<buoy::repository::OracleCommand>(&format!(
                "id = \"{id}\"\nprogram = \"{program}\"\ntimeout_ms = 1000\n"
            ))
            .expect("oracle command")
        })
        .collect::<Vec<_>>();
    let outcome = execute(identity, &commands, Path::new(".")).expect("execution outcome");
    assert_eq!(outcome.static_identity, identity);
    assert_eq!(outcome.exit, EXIT_EXECUTION);
    for ((_, _, expected), receipt) in sentinels.iter().zip(&outcome.receipts) {
        assert_eq!(receipt.verdict, *expected);
    }
    assert!(outcome.report.contains("\"O-ERROR\",\"error\""));
    assert!(
        !outcome.report.contains("stdout") && !outcome.report.contains("elapsed"),
        "canonical execution bytes exclude raw logs and timing: {}",
        outcome.report
    );
    let passing = [toml::from_str::<buoy::repository::OracleCommand>(
        "id = \"O-PASS\"\nprogram = \"builtin:pass\"\ntimeout_ms = 1000\n",
    )
    .expect("oracle command")];
    let passing_outcome = execute(identity, &passing, Path::new(".")).expect("passing outcome");
    assert_eq!(passing_outcome.exit, EXIT_OK);
    assert_eq!(passing_outcome.static_identity, identity);
}

#[test]
fn configuration_defects_fail_at_the_saved_input_boundary() {
    for (bytes, context) in [
        (vec![0xff], "invalid-utf8"),
        (b"not toml".to_vec(), "invalid-toml"),
        (
            b"schema = \"other/v1\"\nselector = []\n".to_vec(),
            "invalid-schema",
        ),
        (
            b"schema = \"chelis-adapter-config/v1\"\nselector = []\n".to_vec(),
            "empty-selectors",
        ),
    ] {
        assert_eq!(
            ChelisConfiguration::parse(&bytes),
            Err(SnapshotError::Configuration(context.to_owned()))
        );
    }
    for (root, dialect, context) in [
        ("chelis", "unknown", "invalid-dialect:unknown"),
        ("/chelis", "surf", "invalid-root:/chelis"),
        ("chelis/../other", "surf", "invalid-root:chelis/../other"),
    ] {
        let bytes = format!(
            "schema = \"chelis-adapter-config/v1\"\n[[selector]]\nroot = \"{root}\"\ndialect = \"{dialect}\"\n"
        );
        assert_eq!(
            ChelisConfiguration::parse(bytes.as_bytes()),
            Err(SnapshotError::Configuration(context.to_owned()))
        );
    }
    let mut noncanonical = String::from(
        "schema = \"chelis-adapter-config/v1\"\nbinding-store = \"chelis/obje\u{301}\"\n",
    );
    noncanonical.push_str("[[selector]]\nroot = \"chelis\"\ndialect = \"surf\"\n");
    assert_eq!(
        ChelisConfiguration::parse(noncanonical.as_bytes()),
        Err(SnapshotError::Configuration(String::from(
            "invalid-binding-store:chelis/obje\u{301}"
        )))
    );
    for field in ["id", "version", "digest", "configuration-digest"] {
        let mut bytes = String::from_utf8(CONFIG.to_vec()).expect("configuration text");
        bytes.push_str("\n[[tool]]\n");
        for key in ["id", "version", "digest", "configuration-digest"] {
            let value = if key == field { "" } else { "saved" };
            writeln!(bytes, "{key} = \"{value}\"").expect("tool field");
        }
        assert_eq!(
            ChelisConfiguration::parse(bytes.as_bytes()),
            Err(SnapshotError::Configuration(String::from("invalid-tool")))
        );
    }
    let mut oversized = String::from_utf8(CONFIG.to_vec()).expect("configuration text");
    oversized.push('#');
    oversized.extend(std::iter::repeat_n('x', 1_048_576));
    let config = ChelisConfiguration::parse(oversized.as_bytes()).expect("large TOML");
    let saved = ChelisSnapshot::from_files(vec![(
        String::from("chelis/source.ch"),
        ChelisDialect::Surf,
        authority_source().into_bytes(),
    )])
    .expect("saved source");
    assert!(matches!(
        static_check(&saved, &config),
        Err(chelis_provenance::command::CommandError::Adapter(_))
    ));
}

#[test]
fn directory_snapshot_preserves_saved_bytes_and_canonical_order() {
    let root = tempfile::tempdir().expect("root");
    let empty = ChelisSnapshot::from_directory(root.path(), &configuration()).expect("empty tree");
    assert!(empty.is_empty());
    assert_eq!(empty.len(), 0);
    std::fs::create_dir_all(root.path().join("chelis")).expect("selector root");
    std::fs::write(root.path().join("chelis/empty.ch"), b"").expect("empty source");
    let empty_file =
        ChelisSnapshot::from_directory(root.path(), &configuration()).expect("empty file");
    assert_eq!(empty_file.len(), 1);
    assert!(!empty_file.is_empty());
    let empty_outcome = static_check(&empty_file, &configuration()).expect("empty outcome");
    assert_eq!(empty_outcome.exit, EXIT_OK);
    assert!(empty_outcome.diagnostics.is_empty());
    assert_eq!(empty_outcome.graph, buoy::core::SpecGraph::default());
    std::fs::create_dir_all(root.path().join("deep/nested")).expect("deep tree");
    std::fs::create_dir_all(root.path().join("chelis/nested")).expect("surf tree");
    std::fs::write(
        root.path().join("deep/nested/source.dp"),
        b"(def deep_subject x)\n",
    )
    .expect("deep source");
    std::fs::write(root.path().join("chelis/nested/z.ch"), authority_source())
        .expect("surf source");
    std::fs::write(
        root.path().join("chelis/a.ch"),
        b"def ordinary(x: f32) -> f32 = x\n",
    )
    .expect("ordinary source");
    std::fs::write(
        root.path().join("chelis/nested/reef.toml"),
        b"saved artifact",
    )
    .expect("saved artifact");
    let saved = ChelisSnapshot::from_directory(root.path(), &configuration()).expect("saved tree");
    assert!(!saved.is_empty());
    assert_eq!(saved.len(), 4);
    assert_eq!(
        saved
            .files()
            .map(|(path, dialect, _)| (path.as_str(), dialect))
            .collect::<Vec<_>>(),
        vec![
            ("chelis/a.ch", ChelisDialect::Surf),
            ("chelis/empty.ch", ChelisDialect::Surf),
            ("chelis/nested/z.ch", ChelisDialect::Surf),
            ("deep/nested/source.dp", ChelisDialect::Deep),
        ]
    );
    assert_eq!(
        saved.artifact("chelis/nested/reef.toml"),
        Some(b"saved artifact".as_slice())
    );
    assert_eq!(saved.artifact("chelis/reef.toml"), None);
    std::fs::write(
        root.path().join("chelis/nested/z.ch"),
        b"def changed(x: f32) -> f32 = x\n",
    )
    .expect("changed source");
    std::fs::write(root.path().join("chelis/nested/reef.toml"), b"new artifact")
        .expect("changed artifact");
    let current =
        ChelisSnapshot::from_directory(root.path(), &configuration()).expect("current tree");
    let saved_outcome = static_check(&saved, &configuration()).expect("saved outcome");
    let current_outcome = static_check(&current, &configuration()).expect("current outcome");
    assert_eq!(
        saved_outcome.graph.atoms[0].meta().id().as_str(),
        "CHELIS-EVAL-DEFAULT"
    );
    assert!(current_outcome.graph.atoms.is_empty());
    assert_eq!(
        saved.artifact("chelis/nested/reef.toml"),
        Some(b"saved artifact".as_slice())
    );
    assert_eq!(
        current.artifact("chelis/nested/reef.toml"),
        Some(b"new artifact".as_slice())
    );
}

#[test]
fn human_rejection_preserves_the_diagnostic_location_and_context() {
    let source = b"-- chelis:provenance/v1 authority\n-- id = HUMAN-ATOM\n-- kind = behavioral\n-- scopes = chelis-core\ndef human(x: f32) -> f32 = x\n";
    let snapshot = ChelisSnapshot::from_files(vec![(
        String::from("chelis/human.ch"),
        ChelisDialect::Surf,
        source.to_vec(),
    )])
    .expect("snapshot");
    let outcome = static_check(&snapshot, &configuration()).expect("static result");
    assert_eq!(outcome.exit, EXIT_STATIC);
    assert_eq!(
        chelis_provenance::command::render_human_static(&outcome),
        "chelis static report\nverdict: reject\nobjects: atoms=0 bindings=0 carriers=0 links=0 surfaces=0 policies=0 derivations=0 oracles=0\nfinding: CHELIS-PROV-MALFORMED-RECORD chelis/human.ch:0..126 missing-field:statement\n",
    );
}

#[test]
fn explicit_atom_mapping_rejects_a_saved_non_authority_record() {
    let source =
        b"-- chelis:provenance/v1 carrier\n-- id = WRONG-KIND\ndef wrong(x: f32) -> f32 = x\n";
    let extracted = extract(ChelisDialect::Surf, source).expect("carrier extraction");
    let record = &extracted.records[0];
    let active = buoy::core::ActiveAlgorithm::new(
        chelis_provenance::command::artifact_digest(b"member")
            .expect("member digest")
            .algorithm(),
    )
    .expect("active member");
    assert_eq!(
        chelis_provenance::map::map_atom_with(active, record),
        Err(chelis_provenance::map::MapError {
            context: String::from("unknown-record-kind:carrier"),
        }),
    );
}
