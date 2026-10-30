//! Compact content-addressed binding contracts.
//!
//! Coverage follows the frozen requirements: the two-line record grammar and
//! its store boundary (`CHELIS-PROV-019`), address integrity against the
//! immutable object, subject currency against the attached declaration
//! (`CHELIS-PROV-020`), and canonical determinism across absolute roots.
//!
//! Content addressing is meaningless under a change-detection member, so this
//! suite exists only in a build with exactly one collision-resistant member. A
//! guard that skipped at run time would report a pass for work it never did.
//! The complementary obligation is covered by `binding_change_detection`.

#![cfg(all(
    any(feature = "blake3-256", feature = "sha-256"),
    not(feature = "xxh3-128")
))]
#![allow(clippy::expect_used, clippy::panic)]

use buoy::adapters::{binding_object_path, canonical_binding_record};
use buoy::core::{
    ActiveAlgorithm, AtomRevisionRef, ImplementationBinding, NavigationKind, SemanticDigest,
    SourceNodeRef, StableId,
};
use chelis_provenance::command::{EXIT_OK, EXIT_STATIC, binding_digest, static_check};
use chelis_provenance::records::ChelisDialect;
use chelis_provenance::snapshot::{ChelisConfiguration, ChelisSnapshot, SnapshotError};

const STORE: &str = "chelis/objects";
const STATEMENT: &str = "The bound subject MUST stay addressed by content.";
const DECLARATION: &str = "def bound_impl(x: f32) -> f32 = x";

fn configuration() -> ChelisConfiguration {
    ChelisConfiguration::parse(
        format!(
            "schema = \"chelis-adapter-config/v1\"\nbinding-store = \"{STORE}\"\n\n[[selector]]\nroot = \"chelis\"\ndialect = \"surf\"\n"
        )
        .as_bytes(),
    )
    .expect("configuration")
}

/// The collision-resistant member this build pins.
fn active() -> ActiveAlgorithm {
    chelis_provenance::command::binding_algorithm()
        .expect("a collision-resistant build addresses content")
}

fn digest(bytes: &[u8]) -> SemanticDigest {
    binding_digest(active(), bytes).expect("binding digest")
}

fn atom_value(statement: &str) -> buoy::core::Atom {
    buoy::core::Atom::new_with(
        active(),
        StableId::new("BIND-ATOM").expect("atom id"),
        buoy::core::LifecycleState::Proposed,
        buoy::core::AtomKind::Behavioral,
        vec![buoy::core::ScopeId::new("chelis-core").expect("scope")],
        statement,
        None,
    )
    .expect("atom value")
}

/// Renders the authority block whose bytes are the requirement source node.
fn authority_block(statement: &str) -> String {
    format!(
        "-- chelis:provenance/v1 authority\n-- id = BIND-ATOM\n-- kind = behavioral\n-- scopes = chelis-core\n-- statement = {statement}\n"
    )
}

struct Scenario {
    source: String,
    object_path: String,
    object_bytes: String,
}

/// Builds one repository whose single binding is current by construction.
fn scenario(
    statement: &str,
    declaration: &str,
    subject_override: Option<SemanticDigest>,
) -> Scenario {
    let atom = atom_value(statement);
    let authority = authority_block(statement);
    let requirement = digest(authority.as_bytes());
    let subject = subject_override.unwrap_or_else(|| digest(declaration.as_bytes()));
    let record = ImplementationBinding::new_with(
        active(),
        StableId::new("BIND-LINK").expect("relation"),
        NavigationKind::Implements,
        SourceNodeRef::new(subject),
        SourceNodeRef::new(requirement),
        AtomRevisionRef::new(
            StableId::new("BIND-ATOM").expect("atom id"),
            atom.revision(),
        ),
    )
    .expect("binding record");
    let object_bytes = canonical_binding_record(&record);
    let object_path = format!("{STORE}/{}", binding_object_path(record.record()));
    let source = format!(
        "{authority}def bind_subject(x: f32) -> f32 = x\n-- chelis:provenance/v1 binding\n-- record = {}\n{declaration}\n",
        record.record().canonical_text(),
    );
    Scenario {
        source,
        object_path,
        object_bytes,
    }
}

fn snapshot_of(scenario: &Scenario, object_bytes: Option<&str>) -> ChelisSnapshot {
    let mut artifacts = Vec::new();
    if let Some(bytes) = object_bytes {
        artifacts.push((scenario.object_path.clone(), bytes.as_bytes().to_vec()));
    }
    ChelisSnapshot::from_files_with_artifacts(
        vec![(
            String::from("chelis/source.ch"),
            ChelisDialect::Surf,
            scenario.source.clone().into_bytes(),
        )],
        artifacts,
    )
    .expect("snapshot")
}

#[test]
fn one_line_resolves_the_full_binding() {
    let scenario = scenario(STATEMENT, DECLARATION, None);
    let snapshot = snapshot_of(&scenario, Some(&scenario.object_bytes));
    let outcome = static_check(&snapshot, &configuration()).expect("static outcome");
    assert_eq!(outcome.exit, EXIT_OK, "{:?}", outcome.diagnostics);
    assert_eq!(
        outcome.graph.implementation_bindings.len(),
        1,
        "the binding materializes into the core graph"
    );
    let binding = &outcome.graph.implementation_bindings[0];
    assert_eq!(binding.meta().id().as_str(), "BIND-LINK");
    assert_eq!(binding.atom().id().as_str(), "BIND-ATOM");
    assert!(
        outcome.report.contains("BIND-LINK"),
        "the report names the materialized binding: {}",
        outcome.report
    );
}

#[test]
fn missing_store_object_fails_closed() {
    let scenario = scenario(STATEMENT, DECLARATION, None);
    let snapshot = snapshot_of(&scenario, None);
    let outcome = static_check(&snapshot, &configuration()).expect("static outcome");
    assert_eq!(outcome.exit, EXIT_STATIC);
    let diagnostic = outcome
        .diagnostics
        .iter()
        .find(|diagnostic| diagnostic.code == "CHELIS-PROV-BINDING-ADDRESS")
        .expect("address integrity must be enforced");
    assert!(
        diagnostic.context.contains("missing-object"),
        "{}",
        diagnostic.context
    );
    assert!(
        outcome.graph.implementation_bindings.is_empty(),
        "a rejected address materializes nothing"
    );
}

#[test]
fn tampered_store_object_dangles() {
    let scenario = scenario(STATEMENT, DECLARATION, None);
    let tampered = scenario
        .object_bytes
        .replace("id = \"BIND-LINK\"", "id = \"BIND-OTHER\"");
    assert_ne!(tampered, scenario.object_bytes, "the tamper applies");
    let snapshot = snapshot_of(&scenario, Some(&tampered));
    let outcome = static_check(&snapshot, &configuration()).expect("static outcome");
    assert_eq!(outcome.exit, EXIT_STATIC);
    let diagnostic = outcome
        .diagnostics
        .iter()
        .find(|diagnostic| diagnostic.code == "CHELIS-PROV-BINDING-ADDRESS")
        .expect("a tampered object must dangle");
    assert!(
        diagnostic.context.contains("object:"),
        "the context names the upstream object failure: {}",
        diagnostic.context
    );
    assert!(outcome.graph.implementation_bindings.is_empty());
}

#[test]
fn malformed_address_is_rejected() {
    let source = format!(
        "{}def bind_subject(x: f32) -> f32 = x\n-- chelis:provenance/v1 binding\n-- record = not-a-digest\n{DECLARATION}\n",
        authority_block(STATEMENT)
    );
    let snapshot = ChelisSnapshot::from_files(vec![(
        String::from("chelis/source.ch"),
        ChelisDialect::Surf,
        source.into_bytes(),
    )])
    .expect("snapshot");
    let outcome = static_check(&snapshot, &configuration()).expect("static outcome");
    assert_eq!(outcome.exit, EXIT_STATIC);
    assert!(
        outcome.diagnostics.iter().any(|diagnostic| diagnostic.code
            == "CHELIS-PROV-BINDING-ADDRESS"
            && diagnostic.context.contains("invalid-address")),
        "{:?}",
        outcome.diagnostics
    );
}

#[test]
fn address_member_mismatch_fails_before_store_lookup() {
    let foreign = if active().algorithm().cargo_feature() == "blake3-256" {
        "sha-256"
    } else {
        "blake3-256"
    };
    let source = format!(
        "{}def bind_subject(x: f32) -> f32 = x\n-- chelis:provenance/v1 binding\n-- record = {foreign}:{}\n{DECLARATION}\n",
        authority_block(STATEMENT),
        "1".repeat(64)
    );
    let snapshot = ChelisSnapshot::from_files(vec![(
        String::from("chelis/source.ch"),
        ChelisDialect::Surf,
        source.into_bytes(),
    )])
    .expect("snapshot");
    let outcome = static_check(&snapshot, &configuration()).expect("static outcome");

    assert_eq!(outcome.exit, EXIT_STATIC);
    assert!(outcome.diagnostics.iter().any(|diagnostic| {
        diagnostic.code == "CHELIS-PROV-BINDING-ADDRESS"
            && diagnostic
                .context
                .starts_with("address-member-mismatch;bound=")
            && diagnostic.context.contains(";current=")
    }));
}

#[test]
fn body_edit_invalidates_the_subject() {
    // Bind against the original declaration, then serve an edited body.
    let edited_declaration = "def bound_impl(x: f32) -> f32 = neg(x)";
    let edited = scenario(
        STATEMENT,
        edited_declaration,
        Some(digest(DECLARATION.as_bytes())),
    );
    let snapshot = snapshot_of(&edited, Some(&edited.object_bytes));
    let outcome = static_check(&snapshot, &configuration()).expect("static outcome");
    assert_eq!(outcome.exit, EXIT_STATIC);
    let diagnostic = outcome
        .diagnostics
        .iter()
        .find(|diagnostic| diagnostic.code == "CHELIS-PROV-BINDING-SUBJECT-STALE")
        .expect("subject currency must be enforced");
    assert_eq!(
        diagnostic.context,
        format!(
            "binding:BIND-LINK;bound={};current={}",
            digest(DECLARATION.as_bytes()).canonical_text(),
            digest(edited_declaration.as_bytes()).canonical_text()
        ),
        "the context carries bound and current subject digests"
    );
    let expected_start = u64::try_from(
        edited
            .source
            .find(edited_declaration)
            .expect("attached declaration"),
    )
    .expect("bounded source");
    assert_eq!(diagnostic.start, expected_start);
    assert_eq!(
        diagnostic.end,
        expected_start + u64::try_from(edited_declaration.len()).expect("bounded declaration")
    );
}

#[test]
fn authority_formatting_invalidates_the_requirement_digest() {
    let original = scenario(STATEMENT, DECLARATION, None);
    let edited_source = original
        .source
        .replacen("-- statement = ", "-- statement =  ", 1);
    assert_ne!(
        edited_source, original.source,
        "the formatting edit applies"
    );
    let edited = Scenario {
        source: edited_source,
        object_path: original.object_path,
        object_bytes: original.object_bytes,
    };
    let outcome = static_check(
        &snapshot_of(&edited, Some(&edited.object_bytes)),
        &configuration(),
    )
    .expect("static outcome");

    assert_eq!(outcome.exit, EXIT_STATIC, "{:?}", outcome.diagnostics);
    assert!(outcome.diagnostics.iter().any(|diagnostic| {
        diagnostic.code == "CHELIS-PROV-BINDING-REQUIREMENT-STALE"
            && diagnostic.context.starts_with("binding:BIND-LINK;bound=")
            && diagnostic.context.contains(";current=")
    }));
    assert!(
        !outcome
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.code == "SPG-REF-DANGLING-OR-STALE"),
        "formatting does not change the canonical atom revision: {:?}",
        outcome.diagnostics
    );
}

#[test]
fn unconfigured_store_rejects_every_binding() {
    let configuration = ChelisConfiguration::parse(
        b"schema = \"chelis-adapter-config/v1\"\n\n[[selector]]\nroot = \"chelis\"\ndialect = \"surf\"\n",
    )
    .expect("configuration");
    let scenario = scenario(STATEMENT, DECLARATION, None);
    let snapshot = snapshot_of(&scenario, Some(&scenario.object_bytes));
    let outcome = static_check(&snapshot, &configuration).expect("static outcome");
    assert_eq!(outcome.exit, EXIT_STATIC);
    assert!(
        outcome.diagnostics.iter().any(|diagnostic| diagnostic.code
            == "CHELIS-PROV-BINDING-ADDRESS"
            && diagnostic.context.contains("store-unconfigured")),
        "{:?}",
        outcome.diagnostics
    );
}

#[test]
fn binding_store_outside_every_selector_is_a_configuration_error() {
    let outside = ChelisConfiguration::parse(
        b"schema = \"chelis-adapter-config/v1\"\nbinding-store = \"objects\"\n\n[[selector]]\nroot = \"chelis\"\ndialect = \"surf\"\n",
    );
    assert!(
        matches!(outside, Err(SnapshotError::Configuration(ref context))
            if context.contains("binding-store-outside-selectors")),
        "{outside:?}"
    );
    for invalid in [
        "/absolute",
        "chelis/../escape",
        "chelis/./objects",
        "chelis\\objects",
        "chelis/",
        "",
    ] {
        let parsed = ChelisConfiguration::parse(
            format!(
                "schema = \"chelis-adapter-config/v1\"\nbinding-store = \"{invalid}\"\n\n[[selector]]\nroot = \"chelis\"\ndialect = \"surf\"\n"
            )
            .as_bytes(),
        );
        assert!(
            matches!(parsed, Err(SnapshotError::Configuration(_))),
            "{invalid} must fail the configuration boundary: {parsed:?}"
        );
    }
}

#[test]
fn deep_binding_markers_remain_unsupported() {
    let configuration = ChelisConfiguration::parse(
        b"schema = \"chelis-adapter-config/v1\"\nbinding-store = \"chelis\"\n\n[[selector]]\nroot = \"chelis\"\ndialect = \"deep\"\n",
    )
    .expect("configuration");
    let source = format!(
        "// chelis:provenance/v1 binding\n// record = {}:{}\n(def bound_impl x)\n",
        active().algorithm().cargo_feature(),
        "0".repeat(64)
    );
    let snapshot = ChelisSnapshot::from_files(vec![(
        String::from("chelis/source.dp"),
        ChelisDialect::Deep,
        source.into_bytes(),
    )])
    .expect("snapshot");
    let outcome = static_check(&snapshot, &configuration).expect("static outcome");

    assert_eq!(outcome.exit, EXIT_STATIC, "{:?}", outcome.diagnostics);
    assert!(
        outcome.diagnostics.iter().any(|diagnostic| {
            diagnostic.code == "CHELIS-PROV-MALFORMED-RECORD"
                && diagnostic.context == "unknown-record-kind:binding"
        }),
        "{:?}",
        outcome.diagnostics
    );
}

#[test]
fn binding_reports_are_root_and_repetition_independent() {
    let scenario = scenario(STATEMENT, DECLARATION, None);
    let first = static_check(
        &snapshot_of(&scenario, Some(&scenario.object_bytes)),
        &configuration(),
    )
    .expect("first outcome");
    let second = static_check(
        &snapshot_of(&scenario, Some(&scenario.object_bytes)),
        &configuration(),
    )
    .expect("second outcome");
    assert_eq!(first.report, second.report);

    let root_a = tempfile::tempdir().expect("root a");
    let root_b = tempfile::tempdir().expect("root b");
    for root in [&root_a, &root_b] {
        let source_path = root.path().join("chelis/source.ch");
        let object_path = root.path().join(&scenario.object_path);
        std::fs::create_dir_all(source_path.parent().expect("source parent"))
            .expect("selector root");
        std::fs::create_dir_all(object_path.parent().expect("object parent")).expect("store root");
        std::fs::write(&source_path, &scenario.source).expect("write source");
        std::fs::write(&object_path, &scenario.object_bytes).expect("write object");
    }
    let configuration = configuration();
    let from_a = ChelisSnapshot::from_directory(root_a.path(), &configuration).expect("snapshot a");
    let from_b = ChelisSnapshot::from_directory(root_b.path(), &configuration).expect("snapshot b");
    let outcome_a = static_check(&from_a, &configuration).expect("outcome a");
    let outcome_b = static_check(&from_b, &configuration).expect("outcome b");
    assert_eq!(outcome_a.report, outcome_b.report);
    assert_eq!(outcome_a.exit, EXIT_OK, "{:?}", outcome_a.diagnostics);
    assert_eq!(outcome_a.report, first.report);
}
