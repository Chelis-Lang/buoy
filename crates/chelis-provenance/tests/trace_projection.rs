//! Deterministic full-chain trace projection contracts.

#![allow(clippy::expect_used, clippy::panic)]

use chelis_provenance::command::{EXIT_INPUT, EXIT_OK, EXIT_STATIC, artifact_digest};
use chelis_provenance::records::ChelisDialect;
use chelis_provenance::snapshot::{ChelisConfiguration, ChelisSnapshot};
use chelis_provenance::trace::trace_projection;
use std::fs;

fn configuration() -> ChelisConfiguration {
    ChelisConfiguration::parse(
        b"schema = \"chelis-adapter-config/v1\"\n\n[[selector]]\nroot = \"chelis\"\ndialect = \"surf\"\n",
    )
    .expect("configuration")
}

fn item_digest(declaration: &str) -> String {
    artifact_digest(declaration.as_bytes())
        .expect("item digest")
        .canonical_text()
}

fn atom_value(statement: &str) -> buoy::core::Atom {
    buoy::core::Atom::new(
        buoy::core::StableId::new("TRACE-ATOM").expect("atom id"),
        buoy::core::LifecycleState::Proposed,
        buoy::core::AtomKind::Behavioral,
        vec![buoy::core::ScopeId::new("chelis-core").expect("scope")],
        statement,
        None,
    )
    .expect("atom value")
}

fn oracle_value(runner_digest: &str, configuration_digest: &str) -> buoy::core::Oracle {
    buoy::core::Oracle::new(
        buoy::core::StableId::new("TRACE-ORACLE").expect("oracle id"),
        buoy::core::OracleKind::Test,
        buoy::core::ToolIdentity::new(
            buoy::core::ScopeId::new("chelis-test").expect("runner scope"),
            "1.0.0",
            buoy::core::SemanticDigest::parse_canonical(runner_digest).expect("runner digest"),
            buoy::core::SemanticDigest::parse_canonical(configuration_digest)
                .expect("runner config"),
        )
        .expect("runner identity"),
        buoy::core::TypedPayload::new(
            buoy::core::SchemaId::new("chelis-oracle-selection/v1").expect("selection schema"),
            b"selection".to_vec(),
        )
        .expect("selection payload"),
        buoy::core::TypedPayload::new(
            buoy::core::SchemaId::new("chelis-oracle-normalization/v1")
                .expect("normalization schema"),
            b"normalization".to_vec(),
        )
        .expect("normalization payload"),
        buoy::core::SemanticDigest::parse_canonical("xxh3-128:00000000000000000000000000000003")
            .expect("oracle configuration"),
        buoy::core::TypedPayload::new(
            buoy::core::SchemaId::new("chelis-oracle-scope/v1").expect("scope schema"),
            b"scope".to_vec(),
        )
        .expect("scope payload"),
        Vec::new(),
    )
    .expect("oracle value")
}

fn scenario(statement: &str, reef: &[u8]) -> String {
    let revision = atom_value(statement).revision().canonical_text();
    let reef_digest = artifact_digest(reef).expect("reef digest").canonical_text();
    let oracle_digest = oracle_value(&reef_digest, &reef_digest)
        .meta()
        .digest()
        .canonical_text();
    let carrier_item = item_digest("def trace_carrier(x: f32) -> f32 = x");
    let link_item = item_digest("def trace_impl(x: f32) -> f32 = x");
    format!(
        "-- chelis:provenance/v1 authority\n-- id = TRACE-ATOM\n-- kind = behavioral\n-- scopes = chelis-core\n-- statement = {statement}\ndef trace_subject(x: f32) -> f32 = x\n\
         -- chelis:provenance/v1 surface\n-- id = TRACE-SURFACE\n-- member-key-schema = chelis-backend-member/v1\n-- disposition-schema = chelis-backend-disposition/v1\n-- generation = 1\n-- members = eval;metal\n-- rows = eval:normative:TRACE-ATOM@{revision};metal:not-applicable\ndef trace_surface(x: f32) -> f32 = x\n\
         -- chelis:provenance/v1 implementation-link\n-- id = TRACE-LINK\n-- atoms = TRACE-ATOM@{revision}\n-- item-digest = {link_item}\n-- navigation = implements\ndef trace_impl(x: f32) -> f32 = x\n\
         -- chelis:provenance/v1 oracle\n-- id = TRACE-ORACLE\n-- oracle-kind = test\n-- runner-id = chelis-test\n-- runner-version = 1.0.0\n-- runner-digest = {reef_digest}\n-- runner-configuration-digest = {reef_digest}\n-- runner-config-paths = chelis/reef.toml\n-- selection-schema = chelis-oracle-selection/v1\n-- selection = selection\n-- normalization-schema = chelis-oracle-normalization/v1\n-- normalization = normalization\n-- configuration-digest = xxh3-128:00000000000000000000000000000003\n-- scope-schema = chelis-oracle-scope/v1\n-- scope = scope\ndef trace_oracle(x: f32) -> f32 = x\n\
         -- chelis:provenance/v1 carrier\n-- id = TRACE-CARRIER\n-- role = positive\n-- atoms = TRACE-ATOM@{revision}\n-- item-digest = {carrier_item}\n-- oracle-id = TRACE-ORACLE\n-- oracle-digest = {oracle_digest}\n-- configuration-digest = xxh3-128:00000000000000000000000000000003\n-- scope-schema = chelis-carrier-scope/v1\n-- scope = trace\ndef trace_carrier(x: f32) -> f32 = x\n"
    )
}

fn snapshot(statement: &str, reef: &[u8]) -> ChelisSnapshot {
    ChelisSnapshot::from_files_with_artifacts(
        vec![(
            String::from("chelis/trace.ch"),
            ChelisDialect::Surf,
            scenario(statement, reef).into_bytes(),
        )],
        vec![(String::from("chelis/reef.toml"), reef.to_vec())],
    )
    .expect("snapshot")
}

const STATEMENT: &str = "The traced subject MUST stay bound end to end.";
const REEF: &[u8] = b"version = \"1.0.0\"\n";

#[test]
fn current_chain_verifies_in_one_command() {
    let outcome = trace_projection(&snapshot(STATEMENT, REEF), &configuration(), "TRACE-ATOM")
        .expect("trace outcome");
    assert_eq!(outcome.exit, EXIT_OK, "{}", outcome.report);
    let report: serde_json::Value = serde_json::from_str(&outcome.report).expect("trace JSON");
    assert_eq!(report["verdict"], "accept");
    assert_eq!(report["atom"]["id"], "TRACE-ATOM");
    assert_eq!(report["surfaces"][0]["id"], "TRACE-SURFACE");
    assert_eq!(report["links"][0]["id"], "TRACE-LINK");
    assert_eq!(report["carriers"][0]["id"], "TRACE-CARRIER");
    #[cfg(feature = "xxh3-128")]
    assert_eq!(
        include_str!("../../../fixtures/chelis-provenance/v1/goldens/trace-projection.json")
            .trim_end(),
        outcome.report
    );
}

#[test]
fn tampered_statement_fails_the_trace() {
    let revision_scenario = scenario(STATEMENT, REEF);
    let tampered = revision_scenario.replace(
        "-- statement = The traced subject MUST stay bound end to end.",
        "-- statement = The traced subject MUST stay bound differently.",
    );
    let snapshot = ChelisSnapshot::from_files_with_artifacts(
        vec![(
            String::from("chelis/trace.ch"),
            ChelisDialect::Surf,
            tampered.into_bytes(),
        )],
        vec![(String::from("chelis/reef.toml"), REEF.to_vec())],
    )
    .expect("snapshot");
    let outcome =
        trace_projection(&snapshot, &configuration(), "TRACE-ATOM").expect("trace outcome");
    assert_eq!(outcome.exit, EXIT_STATIC);
    assert!(outcome.report.contains("\"verdict\":\"reject\""));
    assert!(
        outcome.report.contains("\"matches\":false"),
        "the mismatched hop is named: {}",
        outcome.report
    );
}

#[test]
fn tampered_runner_artifact_fails_the_trace() {
    let snapshot = ChelisSnapshot::from_files_with_artifacts(
        vec![(
            String::from("chelis/trace.ch"),
            ChelisDialect::Surf,
            scenario(STATEMENT, REEF).into_bytes(),
        )],
        vec![(
            String::from("chelis/reef.toml"),
            b"version = \"2.0.0\"\n".to_vec(),
        )],
    )
    .expect("snapshot");
    let outcome =
        trace_projection(&snapshot, &configuration(), "TRACE-ATOM").expect("trace outcome");
    assert_eq!(outcome.exit, EXIT_STATIC);
    assert!(outcome.report.contains("runner-digest:chelis/reef.toml"));
}

#[test]
fn unknown_id_answers_with_the_input_exit_class() {
    let outcome = trace_projection(&snapshot(STATEMENT, REEF), &configuration(), "TRACE-ABSENT")
        .expect("trace outcome");
    assert_eq!(outcome.exit, EXIT_INPUT);
    assert!(outcome.report.contains("unknown-id"));
}

#[test]
fn trace_bytes_are_root_and_repetition_independent() {
    let first = trace_projection(&snapshot(STATEMENT, REEF), &configuration(), "TRACE-ATOM")
        .expect("first outcome");
    let second = trace_projection(&snapshot(STATEMENT, REEF), &configuration(), "TRACE-ATOM")
        .expect("second outcome");
    assert_eq!(first.report, second.report);
    let root_a = tempfile::tempdir().expect("root a");
    let root_b = tempfile::tempdir().expect("root b");
    for root in [&root_a, &root_b] {
        fs::create_dir_all(root.path().join("chelis")).expect("selector root");
        fs::write(
            root.path().join("chelis/trace.ch"),
            scenario(STATEMENT, REEF),
        )
        .expect("write source");
        fs::write(root.path().join("chelis/reef.toml"), REEF).expect("write artifact");
    }
    let from_a = ChelisSnapshot::from_directory(root_a.path(), &configuration()).expect("a");
    let from_b = ChelisSnapshot::from_directory(root_b.path(), &configuration()).expect("b");
    let outcome_a = trace_projection(&from_a, &configuration(), "TRACE-ATOM").expect("a outcome");
    let outcome_b = trace_projection(&from_b, &configuration(), "TRACE-ATOM").expect("b outcome");
    assert_eq!(outcome_a.report, outcome_b.report);
    assert_eq!(outcome_a.report, first.report);
}

#[test]
fn missing_chain_inputs_remain_visible_as_failed_hops() {
    let source = scenario(STATEMENT, REEF);
    let authority_end = source
        .find("-- chelis:provenance/v1 surface")
        .expect("surface marker");
    let oracle_start = source
        .find("-- chelis:provenance/v1 oracle")
        .expect("oracle marker");
    let oracle_end = source
        .find("-- chelis:provenance/v1 carrier")
        .expect("carrier marker");
    for (text, artifacts, collection, expected_hop) in [
        (
            source[authority_end..].to_owned(),
            vec![(String::from("chelis/reef.toml"), REEF.to_vec())],
            "links",
            "atom-revision:TRACE-ATOM",
        ),
        (
            format!("{}{}", &source[..oracle_start], &source[oracle_end..]),
            vec![(String::from("chelis/reef.toml"), REEF.to_vec())],
            "carriers",
            "oracle-digest:TRACE-ORACLE",
        ),
        (
            source.clone(),
            Vec::new(),
            "carriers",
            "runner-digest:chelis/reef.toml",
        ),
        (
            source[source
                .find("-- chelis:provenance/v1 implementation-link")
                .expect("link")..oracle_start]
                .to_owned(),
            Vec::new(),
            "links",
            "atom-revision:TRACE-ATOM",
        ),
        (
            source[oracle_end..].to_owned(),
            Vec::new(),
            "carriers",
            "atom-revision:TRACE-ATOM",
        ),
    ] {
        let saved = ChelisSnapshot::from_files_with_artifacts(
            vec![(
                String::from("chelis/trace.ch"),
                ChelisDialect::Surf,
                text.into_bytes(),
            )],
            artifacts,
        )
        .expect("saved chain");
        let outcome =
            trace_projection(&saved, &configuration(), "TRACE-ATOM").expect("trace outcome");
        assert_eq!(outcome.exit, EXIT_STATIC);
        let report: serde_json::Value = serde_json::from_str(&outcome.report).expect("trace JSON");
        let hop = report[collection][0]["hops"]
            .as_array()
            .expect("trace hops")
            .iter()
            .find(|hop| hop["hop"] == expected_hop)
            .expect("failed hop");
        assert_eq!(hop["current"], "absent");
        assert_eq!(hop["matches"], false);
        assert_ne!(hop["bound"], hop["current"]);
        assert_eq!(report["verdict"], "reject");
    }
}

#[test]
fn runner_configuration_hop_uses_the_second_saved_artifact() {
    let runner_configuration = b"configuration = \"saved\"\n";
    let current = artifact_digest(runner_configuration)
        .expect("configuration digest")
        .canonical_text();
    let old = artifact_digest(REEF)
        .expect("runner digest")
        .canonical_text();
    let old_oracle = oracle_value(&old, &old).meta().digest().canonical_text();
    let current_oracle = oracle_value(&old, &current)
        .meta()
        .digest()
        .canonical_text();
    let source = scenario(STATEMENT, REEF)
        .replace(
            "-- runner-config-paths = chelis/reef.toml",
            "-- runner-config-paths = chelis/reef.toml;chelis/runner.toml",
        )
        .replace(
            &format!("-- runner-configuration-digest = {old}"),
            &format!("-- runner-configuration-digest = {current}"),
        )
        .replace(
            &format!("-- oracle-digest = {old_oracle}"),
            &format!("-- oracle-digest = {current_oracle}"),
        );
    for bytes in [
        runner_configuration.as_slice(),
        b"configuration = \"changed\"\n".as_slice(),
    ] {
        let saved = ChelisSnapshot::from_files_with_artifacts(
            vec![(
                String::from("chelis/trace.ch"),
                ChelisDialect::Surf,
                source.as_bytes().to_vec(),
            )],
            vec![
                (String::from("chelis/reef.toml"), REEF.to_vec()),
                (String::from("chelis/runner.toml"), bytes.to_vec()),
            ],
        )
        .expect("saved chain");
        let outcome =
            trace_projection(&saved, &configuration(), "TRACE-ATOM").expect("trace outcome");
        let report: serde_json::Value = serde_json::from_str(&outcome.report).expect("trace JSON");
        let hops = report["carriers"][0]["hops"]
            .as_array()
            .expect("carrier hops");
        let runner = hops
            .iter()
            .find(|hop| hop["hop"] == "runner-digest:chelis/reef.toml")
            .expect("runner hop");
        let configuration = hops
            .iter()
            .find(|hop| hop["hop"] == "runner-configuration-digest:chelis/runner.toml")
            .expect("configuration hop");
        assert_eq!(runner["matches"], true);
        assert_eq!(configuration["bound"], current);
        assert_eq!(
            configuration["current"],
            artifact_digest(bytes)
                .expect("saved digest")
                .canonical_text()
        );
        assert_eq!(configuration["matches"], bytes == runner_configuration);
        assert_eq!(
            outcome.exit,
            if bytes == runner_configuration {
                EXIT_OK
            } else {
                EXIT_STATIC
            }
        );
    }
}

#[test]
fn trace_retains_stale_carrier_hops_and_ignores_unrelated_oracles() {
    let original = scenario(STATEMENT, REEF);
    let other_oracle = original[original
        .find("-- chelis:provenance/v1 oracle")
        .expect("oracle")
        ..original
            .find("-- chelis:provenance/v1 carrier")
            .expect("carrier")]
        .replace("TRACE-ORACLE", "OTHER-ORACLE")
        .replace("def trace_oracle", "def other_oracle");
    let mut source = original.clone();
    source.push_str(&other_oracle);
    for (text, expected) in [
        (source, None),
        (
            original.replace("-- oracle-kind = test", "-- oracle-kind = property"),
            Some("oracle-digest:TRACE-ORACLE"),
        ),
        (
            original.replace(
                "def trace_carrier(x: f32) -> f32 = x",
                "def trace_carrier(x: f32) -> f32 = x + x",
            ),
            Some("item-digest"),
        ),
    ] {
        let saved = ChelisSnapshot::from_files_with_artifacts(
            vec![(
                String::from("chelis/trace.ch"),
                ChelisDialect::Surf,
                text.into_bytes(),
            )],
            vec![(String::from("chelis/reef.toml"), REEF.to_vec())],
        )
        .expect("saved chain");
        let outcome = trace_projection(&saved, &configuration(), "TRACE-ATOM").expect("trace");
        let report: serde_json::Value = serde_json::from_str(&outcome.report).expect("report");
        let hops = report["carriers"][0]["hops"]
            .as_array()
            .expect("carrier hops");
        if let Some(expected) = expected {
            assert_eq!(outcome.exit, EXIT_STATIC);
            let hop = hops
                .iter()
                .find(|row| row["hop"] == expected)
                .expect("stale hop");
            assert_eq!(hop["matches"], false);
            assert_ne!(hop["current"], "absent");
            assert_ne!(hop["bound"], hop["current"]);
        } else {
            assert_eq!(outcome.exit, EXIT_OK);
            assert!(!hops.iter().any(|row| {
                row["hop"]
                    .as_str()
                    .expect("hop name")
                    .contains("OTHER-ORACLE")
            }));
        }
    }
}

#[test]
fn trace_handles_deep_records_missing_ids_and_optional_runner_paths() {
    let deep_config = ChelisConfiguration::parse(
        b"schema = \"chelis-adapter-config/v1\"\n[[selector]]\nroot = \"deep\"\ndialect = \"deep\"\n",
    ).expect("deep configuration");
    let deep = ChelisSnapshot::from_files(vec![(
        String::from("deep/trace.dp"), ChelisDialect::Deep,
        b"// chelis:provenance/v1 authority\n// id = TRACE-ATOM\n// kind = behavioral\n// scopes = chelis-core\n// statement = The traced subject MUST stay bound end to end.\n(def subject x)\n".to_vec(),
    )]).expect("deep snapshot");
    let outcome = trace_projection(&deep, &deep_config, "TRACE-ATOM").expect("deep trace");
    assert_eq!(outcome.exit, EXIT_OK, "{}", outcome.report);
    let source = scenario(STATEMENT, REEF)
        .replace("-- id = TRACE-CARRIER\n", "")
        .replace("-- runner-config-paths = chelis/reef.toml\n", "");
    let saved = ChelisSnapshot::from_files(vec![(
        String::from("chelis/trace.ch"),
        ChelisDialect::Surf,
        source.into_bytes(),
    )])
    .expect("incomplete snapshot");
    let outcome =
        trace_projection(&saved, &configuration(), "TRACE-ATOM").expect("incomplete trace");
    assert_eq!(outcome.exit, EXIT_STATIC);
    let report: serde_json::Value = serde_json::from_str(&outcome.report).expect("report");
    assert_eq!(report["carriers"], serde_json::json!([]));
    assert_eq!(report["verdict"], "reject");
}

#[cfg(all(
    any(feature = "blake3-256", feature = "sha-256"),
    not(feature = "xxh3-128")
))]
fn compact_binding(authority: &str, declaration: &str) -> buoy::core::ImplementationBinding {
    use buoy::core::{
        AtomRevisionRef, ImplementationBinding, NavigationKind, SourceNodeRef, StableId,
    };
    use chelis_provenance::command::{binding_algorithm, binding_digest};

    let active = binding_algorithm().expect("binding member");
    let atom = atom_value(STATEMENT);
    ImplementationBinding::new_with(
        active,
        StableId::new("TRACE-BINDING").expect("binding"),
        NavigationKind::Implements,
        SourceNodeRef::new(binding_digest(active, declaration.as_bytes()).expect("subject digest")),
        SourceNodeRef::new(binding_digest(active, authority.as_bytes()).expect("authority digest")),
        AtomRevisionRef::new(atom.meta().id().clone(), atom.revision()),
    )
    .expect("binding value")
}

#[cfg(all(
    any(feature = "blake3-256", feature = "sha-256"),
    not(feature = "xxh3-128")
))]
fn assert_stale_carrier_with_binding(
    marker: &str,
    path: &str,
    object: &str,
    configured: &ChelisConfiguration,
) {
    use chelis_provenance::command::static_check;

    let mut source = scenario(STATEMENT, REEF).replace(
        "-- statement = The traced subject MUST stay bound end to end.",
        "-- statement = The traced subject MUST retain its current authority.",
    );
    source.push_str(marker);
    let combined = ChelisSnapshot::from_files_with_artifacts(
        vec![(
            String::from("chelis/combined.ch"),
            ChelisDialect::Surf,
            source.into_bytes(),
        )],
        vec![
            (path.to_owned(), object.as_bytes().to_vec()),
            (String::from("chelis/reef.toml"), REEF.to_vec()),
        ],
    )
    .expect("binding and stale carrier");
    let outcome = static_check(&combined, configured).expect("combined outcome");
    assert!(
        outcome.diagnostics.iter().any(|row| row
            .context
            .starts_with("TRACE-CARRIER;atom=TRACE-ATOM;bound=")
            && row.context.contains(";current=")),
        "{:?}",
        outcome.diagnostics
    );
    assert_eq!(outcome.graph.implementation_bindings.len(), 1);
}

#[cfg(all(
    any(feature = "blake3-256", feature = "sha-256"),
    not(feature = "xxh3-128")
))]
#[test]
fn compact_trace_rejects_missing_authority_and_unresolved_addresses() {
    use buoy::adapters::{binding_object_path, canonical_binding_record};
    use chelis_provenance::command::static_check;
    use std::fmt::Write as _;

    let authority = "-- chelis:provenance/v1 authority\n-- id = TRACE-ATOM\n-- kind = behavioral\n-- scopes = chelis-core\n-- statement = The traced subject MUST stay bound end to end.\n";
    let declaration = "def bound(x: f32) -> f32 = x";
    let binding = compact_binding(authority, declaration);
    let mut marker = String::new();
    writeln!(
        marker,
        "-- chelis:provenance/v1 binding\n-- record = {}\n{declaration}",
        binding.record().canonical_text()
    )
    .expect("marker");
    let mut path = String::from("chelis/objects/");
    path.push_str(&binding_object_path(binding.record()));
    let object = canonical_binding_record(&binding);
    let configured = ChelisConfiguration::parse(
        b"schema = \"chelis-adapter-config/v1\"\nbinding-store = \"chelis/objects\"\n[[selector]]\nroot = \"chelis\"\ndialect = \"surf\"\n",
    ).expect("store configuration");
    let saved = ChelisSnapshot::from_files_with_artifacts(
        vec![(
            String::from("chelis/bound.ch"),
            ChelisDialect::Surf,
            marker.as_bytes().to_vec(),
        )],
        vec![(path.clone(), object.as_bytes().to_vec())],
    )
    .expect("missing authority snapshot");
    let outcome = static_check(&saved, &configured).expect("static missing authority");
    let error = outcome
        .diagnostics
        .iter()
        .find(|row| row.code == "CHELIS-PROV-BINDING-REQUIREMENT-STALE")
        .expect("missing stored authority");
    assert!(
        error.context.contains("current=absent"),
        "{}",
        error.context
    );
    assert_eq!(error.path, "chelis/bound.ch");
    let missing =
        trace_projection(&saved, &configured, "TRACE-ATOM").expect("missing authority trace");
    assert_eq!(missing.exit, EXIT_STATIC, "{}", missing.report);
    let unrelated = trace_projection(&saved, &configured, "OTHER-ATOM").expect("unrelated trace");
    assert_eq!(unrelated.exit, EXIT_INPUT);
    assert_stale_carrier_with_binding(&marker, &path, &object, &configured);
    for (config, text, artifacts) in [
        (
            &configuration(),
            marker.clone(),
            vec![(path, object.into_bytes())],
        ),
        (&configured, marker.clone(), Vec::new()),
        (
            &configured,
            marker.replace(&binding.record().canonical_text(), "garbage"),
            Vec::new(),
        ),
    ] {
        let mut source = String::from(authority);
        source.push_str("def subject(x: f32) -> f32 = x\n");
        source.push_str(&text);
        let saved = ChelisSnapshot::from_files_with_artifacts(
            vec![(
                String::from("chelis/bound.ch"),
                ChelisDialect::Surf,
                source.into_bytes(),
            )],
            artifacts,
        )
        .expect("unresolved snapshot");
        let trace = trace_projection(&saved, config, "TRACE-ATOM").expect("unresolved trace");
        assert_eq!(trace.exit, EXIT_STATIC, "{}", trace.report);
        let report: serde_json::Value = serde_json::from_str(&trace.report).expect("trace JSON");
        assert_eq!(report["verdict"], "reject");
        assert_eq!(report["bindings"], serde_json::json!([]));
    }
}
