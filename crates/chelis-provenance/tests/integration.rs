//! Fixture-driven integration contracts for the Chelis adapter commands.

#![allow(
    clippy::expect_used,
    clippy::format_push_string,
    clippy::panic,
    clippy::too_many_lines
)]

use buoy::core::{
    Atom, AtomKind, AtomRevisionRef, CanonicalValue, LifecycleState, OracleKind, ScopeId,
    SemanticDigest, StableId, ToolIdentity, TypedPayload, semantic_digest,
};
use chelis_provenance::command::{
    ChangeManifest, EXIT_OK, EXIT_STATIC, ReportDiagnostic, StaticOutcome, change_check,
    static_check,
};
use chelis_provenance::records::ChelisDialect;
use chelis_provenance::snapshot::{ChelisConfiguration, ChelisSnapshot};
use std::collections::BTreeMap;
use std::fs;
use std::path::Path;

const FIXTURE_ROOT: &str = "../../fixtures/chelis-provenance/v1";
const TOOL_DIGEST: &str = "xxh3-128:00000000000000000000000000000001";
const TOOL_CONFIGURATION: &str = "xxh3-128:00000000000000000000000000000002";
const ORACLE_CONFIGURATION: &str = "xxh3-128:00000000000000000000000000000003";

#[cfg(any(feature = "blake3-256", feature = "sha-256"))]
#[test]
fn substituted_object_diagnostics_do_not_resolve_the_original_address() {
    use buoy::adapters::{
        binding_object_path, canonical_binding_record, extract_binding_in_mount,
        parse_binding_record,
    };
    use buoy::core::{
        ActiveAlgorithm, DigestAlgorithm, ImplementationBinding, NavigationKind, RepositoryPath,
        SourceNodeRef, semantic_digest_with,
    };

    let member = if cfg!(feature = "blake3-256") {
        DigestAlgorithm::Blake3_256
    } else {
        DigestAlgorithm::Sha256
    };
    let active = ActiveAlgorithm::new(member).expect("available collision-resistant member");
    let subject = semantic_digest_with(active, "diagnostic-subject", Vec::new()).expect("subject");
    let requirement =
        semantic_digest_with(active, "diagnostic-requirement", Vec::new()).expect("requirement");
    let revision =
        semantic_digest_with(active, "diagnostic-revision", Vec::new()).expect("revision");
    let record = |id: &str| {
        ImplementationBinding::new_with(
            active,
            StableId::new(id).expect("relation"),
            NavigationKind::Implements,
            SourceNodeRef::new(subject),
            SourceNodeRef::new(requirement),
            AtomRevisionRef::new(StableId::new("DIAGNOSTIC-ATOM").expect("atom"), revision),
        )
        .expect("binding record")
    };
    let original = record("DIAGNOSTIC-ORIGINAL");
    let substituted = record("DIAGNOSTIC-SUBSTITUTED");
    let source = canonical_binding_record(&substituted);
    let parsed = parse_binding_record(active, source.as_bytes()).expect("substituted content");
    assert_eq!(parsed.meta().id(), substituted.meta().id());
    assert_ne!(parsed.record(), original.record());
    let old_path = RepositoryPath::new(&binding_object_path(original.record())).expect("old path");
    let stored = extract_binding_in_mount(active, None, &old_path, source.as_bytes());
    assert!(stored.stored.is_none());
    let [diagnostic] = stored.diagnostics.as_slice() else {
        panic!("one store rejection");
    };
    assert_eq!(diagnostic.code, "CAB-OBJECT-PATH-DIGEST");
    assert_eq!(
        parse_binding_record(active, format!("# comment\n{source}").as_bytes()),
        None,
        "diagnostic parsing still requires canonical bytes"
    );
}

fn declaration_item_digest(declaration: &str) -> String {
    artifact_digest(declaration.as_bytes())
}

fn artifact_digest(bytes: &[u8]) -> String {
    semantic_digest(
        "chelis-raw-artifact",
        vec![(1, CanonicalValue::Bytes(bytes.to_vec()))],
    )
    .expect("artifact digest")
    .canonical_text()
}

fn record(
    dialect: ChelisDialect,
    kind: &str,
    fields: &[(&str, String)],
    declaration: &str,
) -> String {
    let marker = dialect.comment_marker();
    let mut text = format!("{marker} chelis:provenance/v1 {kind}\n");
    for (key, value) in fields {
        text.push_str(&format!("{marker} {key} = {value}\n"));
    }
    text.push_str(declaration);
    text.push('\n');
    text
}

fn oracle_value() -> buoy::core::Oracle {
    buoy::core::Oracle::new(
        StableId::new("CHELIS-CONSISTENCY-ORACLE").expect("oracle id"),
        OracleKind::DerivationConsistency,
        ToolIdentity::new(
            ScopeId::new("chelis-runner").expect("runner scope"),
            "1.0.0",
            SemanticDigest::parse_canonical(TOOL_DIGEST).expect("runner digest"),
            SemanticDigest::parse_canonical(TOOL_CONFIGURATION).expect("runner configuration"),
        )
        .expect("runner identity"),
        TypedPayload::new(
            buoy::core::SchemaId::new("chelis-oracle-selection/v1").expect("selection schema"),
            b"selection".to_vec(),
        )
        .expect("selection payload"),
        TypedPayload::new(
            buoy::core::SchemaId::new("chelis-oracle-normalization/v1")
                .expect("normalization schema"),
            b"normalization".to_vec(),
        )
        .expect("normalization payload"),
        SemanticDigest::parse_canonical(ORACLE_CONFIGURATION).expect("oracle configuration"),
        TypedPayload::new(
            buoy::core::SchemaId::new("chelis-oracle-scope/v1").expect("scope schema"),
            b"scope".to_vec(),
        )
        .expect("scope payload"),
        Vec::new(),
    )
    .expect("oracle value")
}

fn oracle_record(dialect: ChelisDialect) -> String {
    let oracle = oracle_value();
    record(
        dialect,
        "oracle",
        &[
            ("id", String::from("CHELIS-CONSISTENCY-ORACLE")),
            ("oracle-kind", String::from("derivation-consistency")),
            ("runner-id", String::from("chelis-runner")),
            ("runner-version", String::from("1.0.0")),
            ("runner-digest", String::from(TOOL_DIGEST)),
            (
                "runner-configuration-digest",
                String::from(TOOL_CONFIGURATION),
            ),
            (
                "selection-schema",
                String::from("chelis-oracle-selection/v1"),
            ),
            ("selection", String::from("selection")),
            (
                "normalization-schema",
                String::from("chelis-oracle-normalization/v1"),
            ),
            ("normalization", String::from("normalization")),
            ("configuration-digest", String::from(ORACLE_CONFIGURATION)),
            ("scope-schema", String::from("chelis-oracle-scope/v1")),
            ("scope", String::from("scope")),
        ],
        "def oracle_marker(x: f32) -> f32 = x",
    ) + &format!(
        "-- oracle digest {}\n",
        oracle.meta().digest().canonical_text()
    )
}

fn oracle_digest() -> String {
    oracle_value().meta().digest().canonical_text()
}

struct DerivationScenario {
    source_bytes: Vec<u8>,
    target_bytes: Vec<u8>,
    tool_version: String,
    tool_configuration: String,
    include_target: bool,
}

impl DerivationScenario {
    fn current() -> Self {
        Self {
            source_bytes: b"source-data".to_vec(),
            target_bytes: b"target-data".to_vec(),
            tool_version: String::from("1.0.0"),
            tool_configuration: String::from(TOOL_CONFIGURATION),
            include_target: true,
        }
    }

    fn run(&self) -> (StaticOutcome, serde_json::Value) {
        self.run_with(None, false, false)
    }

    fn run_with(
        &self,
        tool: Option<&str>,
        missing_source: bool,
        duplicate: bool,
    ) -> (StaticOutcome, serde_json::Value) {
        self.run_with_records(tool, missing_source, duplicate, "")
    }

    fn run_with_records(
        &self,
        tool: Option<&str>,
        missing_source: bool,
        duplicate: bool,
        extra: &str,
    ) -> (StaticOutcome, serde_json::Value) {
        let recorded_source = artifact_digest(b"source-data");
        let recorded_target = artifact_digest(b"target-data");
        let derivation = record(
            ChelisDialect::Surf,
            "derivation",
            &[
                ("id", String::from("CHELIS-STD-BUNDLE-DERIVATION")),
                (
                    "sources",
                    format!("chelis/source-data.txt|text/plain|raw-bytes|{recorded_source}"),
                ),
                (
                    "targets",
                    format!("chelis/target-data.txt|text/plain|raw-bytes|{recorded_target}"),
                ),
                ("tool-id", String::from("chelis-generator")),
                ("tool-version", String::from("1.0.0")),
                ("tool-digest", String::from(TOOL_DIGEST)),
                (
                    "tool-configuration-digest",
                    String::from(TOOL_CONFIGURATION),
                ),
                ("configuration-digest", String::from(ORACLE_CONFIGURATION)),
                ("oracle-id", String::from("CHELIS-CONSISTENCY-ORACLE")),
                ("oracle-digest", oracle_digest()),
                ("relation", String::from("generated-from")),
                ("scope-schema", String::from("chelis-derivation-scope/v1")),
                ("scope", String::from("std-bundle")),
            ],
            "def regenerate(x: f32) -> f32 = x",
        );
        let mut source_text = oracle_record(ChelisDialect::Surf);
        source_text.push_str(&derivation);
        if duplicate {
            source_text.push_str(
                &derivation
                    .replace("CHELIS-STD-BUNDLE-DERIVATION", "SECOND-DERIVATION")
                    .replace("def regenerate", "def regenerate_second"),
            );
        }
        source_text.push_str(extra);
        let mut artifacts = vec![(
            String::from("chelis/source-data.txt"),
            self.source_bytes.clone(),
        )];
        if self.include_target {
            artifacts.push((
                String::from("chelis/target-data.txt"),
                self.target_bytes.clone(),
            ));
        }
        if missing_source {
            artifacts.retain(|(path, _)| path != "chelis/source-data.txt");
        }
        let snapshot = ChelisSnapshot::from_files_with_artifacts(
            vec![(
                String::from("chelis/provenance.ch"),
                ChelisDialect::Surf,
                source_text.into_bytes(),
            )],
            artifacts,
        )
        .expect("snapshot");
        let mut configuration_text = String::from(
            "schema = \"chelis-adapter-config/v1\"\n[[selector]]\nroot = \"chelis\"\ndialect = \"surf\"\n",
        );
        if let Some(tool) = tool {
            configuration_text.push_str(tool);
        } else {
            use std::fmt::Write as _;
            write!(configuration_text, "\n[[tool]]\nid = \"chelis-generator\"\nversion = \"{}\"\ndigest = \"{TOOL_DIGEST}\"\nconfiguration-digest = \"{}\"\n", self.tool_version, self.tool_configuration).expect("tool configuration");
        }
        let configuration =
            ChelisConfiguration::parse(configuration_text.as_bytes()).expect("configuration");
        let outcome = static_check(&snapshot, &configuration).expect("static outcome");
        let advisory = chelis_provenance::advisory::advisory_report(&snapshot, &outcome)
            .expect("advisory report");
        (
            outcome,
            serde_json::from_str(&advisory).expect("advisory JSON"),
        )
    }
}

fn codes(diagnostics: &[ReportDiagnostic]) -> Vec<&str> {
    diagnostics
        .iter()
        .map(|diagnostic| diagnostic.code.as_str())
        .collect()
}

fn read_cases(name: &str) -> Vec<toml::Table> {
    let path = Path::new(FIXTURE_ROOT).join("cases").join(name);
    let source = fs::read_to_string(path).expect("case file");
    let document = source.parse::<toml::Table>().expect("case TOML");
    document
        .get("case")
        .and_then(toml::Value::as_array)
        .expect("case array")
        .iter()
        .map(|value| value.as_table().expect("case table").clone())
        .collect()
}

fn case_text<'a>(case: &'a toml::Table, key: &str) -> &'a str {
    case.get(key)
        .and_then(toml::Value::as_str)
        .expect("case text field")
}

fn case_diagnostic(case: &toml::Table) -> Option<String> {
    case.get("diagnostics")
        .and_then(toml::Value::as_array)
        .expect("diagnostics array")
        .first()
        .map(|value| value.as_str().expect("diagnostic text").to_owned())
}

#[test]
fn derivation_fixture_cases_execute() {
    let mut cases = read_cases("derivation-artifacts.toml");
    cases.extend(read_cases("derivations.toml"));
    let executed = cases
        .iter()
        .filter(|case| case_text(case, "operation") == "derivation-freshness")
        .count();
    assert!(executed >= 24, "expected frozen derivation cases");
    for case in &cases {
        if case_text(case, "operation") != "derivation-freshness" {
            continue;
        }
        let id = case_text(case, "id");
        let expected = case_diagnostic(case);
        let mut scenario = DerivationScenario::current();
        match expected.as_deref() {
            None => {}
            Some("CHELIS-PROV-DERIVATION-SOURCE-STALE") => {
                scenario.source_bytes = b"source-data-drifted".to_vec();
            }
            Some("CHELIS-PROV-DERIVATION-TARGET-STALE") => {
                scenario.target_bytes = b"target-data-drifted".to_vec();
            }
            Some("CHELIS-PROV-DERIVATION-TOOL-STALE") => {
                scenario.tool_version = String::from("1.1.0");
            }
            Some("CHELIS-PROV-DERIVATION-CONFIGURATION-STALE") => {
                scenario.tool_configuration =
                    String::from("xxh3-128:000000000000000000000000000000ff");
            }
            Some("CHELIS-PROV-DERIVATION-STALE") => {
                scenario.include_target = false;
            }
            Some(other) => panic!("{id}: unexpected frozen diagnostic {other}"),
        }
        let (outcome, advisory) = scenario.run();
        match expected {
            None => {
                assert_eq!(outcome.exit, EXIT_OK, "{id}: {:?}", outcome.diagnostics);
                assert!(outcome.diagnostics.is_empty(), "{id}");
                assert!(!outcome.graph.derivations.is_empty(), "{id}");
                assert_eq!(advisory["stale_derivations"], serde_json::json!([]), "{id}");
            }
            Some(code) => {
                assert_eq!(outcome.exit, EXIT_STATIC, "{id}");
                assert!(
                    codes(&outcome.diagnostics).contains(&code.as_str()),
                    "{id}: {:?}",
                    outcome.diagnostics
                );
                let stale = advisory["stale_derivations"]
                    .as_array()
                    .expect("stale derivations");
                assert!(
                    stale.iter().any(|entry| {
                        let text = entry.as_str().expect("derivation context");
                        text.starts_with(&format!("{code}:"))
                            && text.contains("CHELIS-STD-BUNDLE-DERIVATION")
                    }),
                    "{id}: {advisory}"
                );
            }
        }
    }
}

#[test]
fn derivations_preserve_missing_artifacts_tool_identity_and_shared_artifact_entries() {
    let scenario = DerivationScenario::current();
    let (missing, _) = scenario.run_with(None, true, false);
    assert!(
        missing
            .diagnostics
            .iter()
            .any(|row| row.path == "chelis/source-data.txt"
                && row.code == "CHELIS-PROV-DERIVATION-STALE"
                && row.context.contains("current=absent"))
    );
    let (baseline, _) = scenario.run();
    let (shared, _) = scenario.run_with(None, false, true);
    assert_eq!(shared.exit, baseline.exit);
    assert_eq!(shared.graph.derivations.len(), 2);
    assert!(!codes(&shared.diagnostics).contains(&"CHELIS-PROV-MALFORMED-RECORD"));
    let different = "\n[[tool]]\nid = \"chelis-generator\"\nversion = \"1.0.0\"\ndigest = \"xxh3-128:00000000000000000000000000000004\"\nconfiguration-digest = \"xxh3-128:00000000000000000000000000000002\"\n";
    let (stale, _) = scenario.run_with(Some(different), false, false);
    assert!(codes(&stale.diagnostics).contains(&"CHELIS-PROV-DERIVATION-TOOL-STALE"));
    for tool in [
        "",
        "\n[[tool]]\nid = \"chelis-generator\"\nversion = \"1.0.0\"\ndigest = \"garbage\"\nconfiguration-digest = \"xxh3-128:00000000000000000000000000000002\"\n",
        "\n[[tool]]\nid = \"chelis-generator\"\nversion = \"1.0.0\"\ndigest = \"xxh3-128:00000000000000000000000000000001\"\nconfiguration-digest = \"garbage\"\n",
    ] {
        let (unconfigured, _) = scenario.run_with(Some(tool), false, false);
        assert_eq!(unconfigured.graph, baseline.graph);
        assert!(!codes(&unconfigured.diagnostics).contains(&"CHELIS-PROV-DERIVATION-TOOL-STALE"));
    }
    let stale_carrier = record(
        ChelisDialect::Surf,
        "carrier",
        &[
            ("id", String::from("STALE-CARRIER")),
            ("role", String::from("positive")),
            ("atoms", format!("ABSENT-ATOM@{TOOL_DIGEST}")),
            (
                "item-digest",
                declaration_item_digest("def stale(x: f32) -> f32 = x"),
            ),
            ("oracle-id", String::from("CHELIS-CONSISTENCY-ORACLE")),
            ("oracle-digest", String::from(TOOL_DIGEST)),
            ("configuration-digest", String::from(ORACLE_CONFIGURATION)),
            ("scope-schema", String::from("scope/v1")),
            ("scope", String::from("stale")),
        ],
        "def stale(x: f32) -> f32 = x",
    );
    let (combined, _) = scenario.run_with_records(None, false, false, &stale_carrier);
    assert!(
        combined
            .diagnostics
            .iter()
            .any(|row| row.context.starts_with("STALE-CARRIER;")
                && row
                    .context
                    .contains("oracle=CHELIS-CONSISTENCY-ORACLE;bound=")
                && row
                    .context
                    .contains(&format!(";current={}", oracle_digest()))),
        "{:?}",
        combined.diagnostics
    );
}

#[test]
fn two_changed_atoms_share_one_impacted_carrier_and_formal_property() {
    use std::fmt::Write as _;
    let build = |statement: &str| {
        let mut source = oracle_record(ChelisDialect::Surf);
        let mut atoms = String::new();
        for id in ["CHANGE-A", "CHANGE-B"] {
            let atom = Atom::new(
                StableId::new(id).expect("atom id"),
                LifecycleState::Proposed,
                AtomKind::Behavioral,
                vec![ScopeId::new("chelis-core").expect("scope")],
                statement,
                None,
            )
            .expect("atom value");
            if !atoms.is_empty() {
                atoms.push(';');
            }
            write!(atoms, "{id}@{}", atom.revision().canonical_text()).expect("atom reference");
            let mut declaration = String::new();
            write!(
                declaration,
                "def subject_{}(x: f32) -> f32 = x",
                if id == "CHANGE-A" { "a" } else { "b" }
            )
            .expect("declaration");
            source.push_str(&record(
                ChelisDialect::Surf,
                "authority",
                &[
                    ("id", id.to_owned()),
                    ("kind", String::from("behavioral")),
                    ("scopes", String::from("chelis-core")),
                    ("statement", statement.to_owned()),
                ],
                &declaration,
            ));
        }
        source.push_str(&record(
            ChelisDialect::Surf,
            "carrier",
            &[
                ("id", String::from("SHARED-CARRIER")),
                ("role", String::from("positive")),
                ("atoms", atoms.clone()),
                (
                    "item-digest",
                    declaration_item_digest("def shared(x: f32) -> f32 = x"),
                ),
                ("oracle-id", String::from("CHELIS-CONSISTENCY-ORACLE")),
                ("oracle-digest", oracle_digest()),
                ("configuration-digest", String::from(ORACLE_CONFIGURATION)),
                ("scope-schema", String::from("scope/v1")),
                ("scope", String::from("shared")),
            ],
            "def shared(x: f32) -> f32 = x",
        ));
        source.push_str(&record(
            ChelisDialect::Surf,
            "formal-property",
            &[
                ("id", String::from("SHARED-PROPERTY")),
                ("atoms", atoms),
                ("property-schema", String::from("property/v1")),
                ("property", String::from("The shared function is total")),
                ("abstraction-schema", String::from("abstraction/v1")),
                ("abstraction", String::from("Identity function")),
                ("approval-authority", String::from("chelis-core")),
                ("approval-record", String::from("reviews/property.md")),
                ("approval-digest", String::from(TOOL_DIGEST)),
            ],
            "def property_marker(x: f32) -> f32 = x",
        ));
        let saved = ChelisSnapshot::from_files(vec![(
            String::from("chelis/change.ch"),
            ChelisDialect::Surf,
            source.into_bytes(),
        )])
        .expect("change snapshot");
        let config = ChelisConfiguration::parse(
            b"schema = \"chelis-adapter-config/v1\"\n[[selector]]\nroot = \"chelis\"\ndialect = \"surf\"\n",
        ).expect("configuration");
        static_check(&saved, &config).expect("static change")
    };
    let base = build("The shared function MUST return the input.");
    let head = build("The shared function MUST return the input without changes.");
    let missing = change_check(&base, &head, &manifest(&[])).expect("missing dispositions");
    for id in [
        "evidence-carrier:SHARED-CARRIER",
        "formal-property:SHARED-PROPERTY",
    ] {
        assert_eq!(
            missing
                .diagnostics
                .iter()
                .filter(
                    |row| row.code == "CHELIS-PROV-IMPACT-DISPOSITION-MISSING" && row.context == id
                )
                .count(),
            1,
            "{:?}",
            missing.diagnostics
        );
    }
    let complete = manifest(&[
        ("atom", "CHANGE-A", "revised"),
        ("atom", "CHANGE-B", "revised"),
        ("evidence-carrier", "SHARED-CARRIER", "revised"),
        ("formal-property", "SHARED-PROPERTY", "revised"),
    ]);
    let accepted = change_check(&base, &head, &complete).expect("complete dispositions");
    assert_eq!(accepted.exit, EXIT_OK, "{:?}", accepted.diagnostics);
}

struct ChangeScenario {
    statement: &'static str,
    carrier_atom_current: bool,
}

fn atom_value(statement: &str) -> Atom {
    Atom::new(
        StableId::new("CHELIS-BACKEND-SELECTION").expect("atom id"),
        LifecycleState::Proposed,
        AtomKind::Behavioral,
        vec![ScopeId::new("chelis-core").expect("scope")],
        statement,
        None,
    )
    .expect("atom value")
}

impl ChangeScenario {
    fn build(&self) -> StaticOutcome {
        let atom = atom_value(self.statement);
        let bound_revision = if self.carrier_atom_current {
            atom.revision()
        } else {
            atom_value("The compiler MUST select the eval backend by default.").revision()
        };
        let atom_reference = format!(
            "CHELIS-BACKEND-SELECTION@{}",
            bound_revision.canonical_text()
        );
        let carrier = buoy::core::EvidenceCarrier::new(
            StableId::new("CHELIS-BACKEND-CARRIER").expect("carrier id"),
            buoy::core::EvidenceRole::Positive,
            vec![AtomRevisionRef::new(
                StableId::new("CHELIS-BACKEND-SELECTION").expect("atom id"),
                bound_revision,
            )],
            SemanticDigest::parse_canonical(&declaration_item_digest(
                "def backend_positive(x: f32) -> f32 = x",
            ))
            .expect("item digest"),
            buoy::core::ObjectRef::new(
                buoy::core::ObjectKind::Oracle,
                StableId::new("CHELIS-CONSISTENCY-ORACLE").expect("oracle id"),
                oracle_value().meta().digest(),
            ),
            SemanticDigest::parse_canonical(ORACLE_CONFIGURATION).expect("configuration"),
            Vec::new(),
            TypedPayload::new(
                buoy::core::SchemaId::new("chelis-carrier-scope/v1").expect("scope schema"),
                b"backend-selection".to_vec(),
            )
            .expect("scope payload"),
            Vec::new(),
            Vec::new(),
        )
        .expect("carrier value");
        let carrier_reference = format!(
            "evidence-carrier:CHELIS-BACKEND-CARRIER@{}",
            carrier.meta().digest().canonical_text()
        );
        let statement_field = self.statement.to_owned();
        let source_text = format!(
            "{}{}{}{}",
            oracle_record(ChelisDialect::Surf),
            record(
                ChelisDialect::Surf,
                "authority",
                &[
                    ("id", String::from("CHELIS-BACKEND-SELECTION")),
                    ("kind", String::from("behavioral")),
                    ("scopes", String::from("chelis-core")),
                    ("statement", statement_field),
                ],
                "def backend(x: f32) -> f32 = x",
            ),
            record(
                ChelisDialect::Surf,
                "carrier",
                &[
                    ("id", String::from("CHELIS-BACKEND-CARRIER")),
                    ("role", String::from("positive")),
                    ("atoms", atom_reference),
                    (
                        "item-digest",
                        declaration_item_digest("def backend_positive(x: f32) -> f32 = x"),
                    ),
                    ("oracle-id", String::from("CHELIS-CONSISTENCY-ORACLE")),
                    ("oracle-digest", oracle_digest()),
                    ("configuration-digest", String::from(ORACLE_CONFIGURATION)),
                    ("scope-schema", String::from("chelis-carrier-scope/v1")),
                    ("scope", String::from("backend-selection")),
                ],
                "def backend_positive(x: f32) -> f32 = x",
            ),
            record(
                ChelisDialect::Surf,
                "waiver",
                &[
                    ("id", String::from("CHELIS-BACKEND-WAIVER")),
                    (
                        "obligations",
                        format!("CHELIS-POLICY:execution:{carrier_reference}"),
                    ),
                    ("objects", carrier_reference.clone()),
                    ("rationale", String::from("bounded process continuation")),
                    ("owner", String::from("chelis-core")),
                    ("discharge-schema", String::from("chelis-discharge/v1")),
                    ("discharge", String::from("carrier passes")),
                    ("approval-authority", String::from("chelis-core")),
                    ("approval-record", String::from("reviews/waiver.md")),
                    ("approval-digest", String::from(TOOL_DIGEST)),
                ],
                "def backend_waiver(x: f32) -> f32 = x",
            ),
        );
        let snapshot = ChelisSnapshot::from_files(vec![(
            String::from("chelis/provenance.ch"),
            ChelisDialect::Surf,
            source_text.into_bytes(),
        )])
        .expect("snapshot");
        let configuration = ChelisConfiguration::parse(
            b"schema = \"chelis-adapter-config/v1\"\n\n[[selector]]\nroot = \"chelis\"\ndialect = \"surf\"\n",
        )
        .expect("configuration");
        static_check(&snapshot, &configuration).expect("static outcome")
    }
}

fn manifest(rows: &[(&str, &str, &str)]) -> ChangeManifest {
    let mut text = String::from("schema = \"chelis-change-manifest/v1\"\n");
    for (kind, id, disposition) in rows {
        text.push_str(&format!(
            "\n[[disposition]]\nobject-kind = \"{kind}\"\nobject-id = \"{id}\"\nkind = \"{disposition}\"\n"
        ));
    }
    ChangeManifest::parse(text.as_bytes()).expect("manifest")
}

#[test]
fn change_manifest_fixture_cases_execute() {
    let cases = read_cases("change-manifests.toml");
    assert!(
        cases
            .iter()
            .any(|case| case_text(case, "id") == "change-base-head-transitive")
    );
    assert!(
        cases
            .iter()
            .any(|case| case_text(case, "id") == "change-base-head-provider-neutral")
    );
    let base = ChangeScenario {
        statement: "The compiler MUST select the eval backend by default.",
        carrier_atom_current: true,
    }
    .build();
    let head = ChangeScenario {
        statement: "The compiler MUST select the eval backend unless configured.",
        carrier_atom_current: true,
    }
    .build();
    let complete = manifest(&[
        ("atom", "CHELIS-BACKEND-SELECTION", "revised"),
        ("evidence-carrier", "CHELIS-BACKEND-CARRIER", "revised"),
        ("waiver", "CHELIS-BACKEND-WAIVER", "revised"),
    ]);
    let outcome = change_check(&base, &head, &complete).expect("change outcome");
    assert_eq!(outcome.exit, EXIT_OK, "{:?}", outcome.diagnostics);
    assert!(
        outcome.report.contains("waiver:CHELIS-BACKEND-WAIVER"),
        "two-hop transitive impact must be visible: {}",
        outcome.report
    );

    let missing = manifest(&[("atom", "CHELIS-BACKEND-SELECTION", "revised")]);
    let outcome = change_check(&base, &head, &missing).expect("missing outcome");
    assert!(
        codes(&outcome.diagnostics).contains(&"CHELIS-PROV-IMPACT-DISPOSITION-MISSING"),
        "{:?}",
        outcome.diagnostics
    );

    let duplicated = manifest(&[
        ("atom", "CHELIS-BACKEND-SELECTION", "revised"),
        ("atom", "CHELIS-BACKEND-SELECTION", "reaffirmed"),
        ("evidence-carrier", "CHELIS-BACKEND-CARRIER", "revised"),
        ("waiver", "CHELIS-BACKEND-WAIVER", "revised"),
    ]);
    let outcome = change_check(&base, &head, &duplicated).expect("duplicate outcome");
    assert!(
        codes(&outcome.diagnostics).contains(&"CHELIS-PROV-IMPACT-DISPOSITION-DUPLICATE"),
        "{:?}",
        outcome.diagnostics
    );

    let stale_head = ChangeScenario {
        statement: "The compiler MUST select the eval backend unless configured.",
        carrier_atom_current: false,
    }
    .build();
    let reaffirmed = manifest(&[
        ("atom", "CHELIS-BACKEND-SELECTION", "revised"),
        ("evidence-carrier", "CHELIS-BACKEND-CARRIER", "reaffirmed"),
        ("waiver", "CHELIS-BACKEND-WAIVER", "revised"),
    ]);
    let outcome = change_check(&base, &stale_head, &reaffirmed).expect("reaffirmed outcome");
    assert!(
        codes(&outcome.diagnostics).contains(&"CHELIS-PROV-REAFFIRMATION-REVIEW-ONLY"),
        "{:?}",
        outcome.diagnostics
    );

    let first = change_check(&base, &head, &complete).expect("first neutral run");
    let second = change_check(&base, &head, &complete).expect("second neutral run");
    assert_eq!(
        first.report, second.report,
        "provider metadata and repetition cannot alter canonical report bytes"
    );
    assert_eq!(base.report, {
        let rebuilt = ChangeScenario {
            statement: "The compiler MUST select the eval backend by default.",
            carrier_atom_current: true,
        }
        .build();
        rebuilt.report
    });
}

#[test]
fn fresh_change_dispositions_accept_reaffirmed_atoms_and_carriers() {
    assert!(matches!(
        ChangeManifest::parse(&[0xff]),
        Err(chelis_provenance::command::CommandError::Encoding)
    ));
    assert!(matches!(
        ChangeManifest::parse(b"schema = \"other/v1\"\n"),
        Err(chelis_provenance::command::CommandError::Encoding)
    ));
    let base = ChangeScenario {
        statement: "The compiler MUST select the eval backend by default.",
        carrier_atom_current: true,
    }
    .build();
    let head = ChangeScenario {
        statement: "The compiler MUST select the eval backend unless configured.",
        carrier_atom_current: true,
    }
    .build();
    let rows = manifest(&[
        ("atom", "CHELIS-BACKEND-SELECTION", "reaffirmed"),
        ("evidence-carrier", "CHELIS-BACKEND-CARRIER", "reaffirmed"),
        ("waiver", "CHELIS-BACKEND-WAIVER", "revised"),
    ]);
    let outcome = change_check(&base, &head, &rows).expect("fresh reaffirmation");
    assert_eq!(outcome.exit, EXIT_OK, "{:?}", outcome.diagnostics);
    assert!(!codes(&outcome.diagnostics).contains(&"CHELIS-PROV-REAFFIRMATION-REVIEW-ONLY"));
}

fn policy_source(enforcement: &str, roles: &str) -> StaticOutcome {
    policy_source_with_scope(enforcement, roles, "backend-selection")
}

fn policy_source_with_scope(enforcement: &str, roles: &str, scope: &str) -> StaticOutcome {
    let source_text = record(
        ChelisDialect::Surf,
        "policy",
        &[
            ("id", String::from("CHELIS-BACKEND-POLICY")),
            ("scope-schema", String::from("chelis-policy-scope/v1")),
            ("scope", String::from(scope)),
            ("required-roles", String::from(roles)),
            ("enforcement", String::from(enforcement)),
            ("ratchet", String::from("chelis-backend-ratchet")),
            ("selector-root", String::from("chelis")),
            ("selector-adapter", String::from("chelis-surf/v1")),
            ("selector-includes", String::from("**/*.ch")),
            ("approval-authority", String::from("chelis-core")),
            ("approval-record", String::from("reviews/policy.md")),
            ("approval-digest", String::from(TOOL_DIGEST)),
        ],
        "def policy_marker(x: f32) -> f32 = x",
    );
    let snapshot = ChelisSnapshot::from_files(vec![(
        String::from("chelis/policy.ch"),
        ChelisDialect::Surf,
        source_text.into_bytes(),
    )])
    .expect("snapshot");
    let configuration = ChelisConfiguration::parse(
        b"schema = \"chelis-adapter-config/v1\"\n\n[[selector]]\nroot = \"chelis\"\ndialect = \"surf\"\n",
    )
    .expect("configuration");
    static_check(&snapshot, &configuration).expect("static outcome")
}

#[test]
fn policy_ratchets_reject_removed_or_changed_scope_and_accept_equal_or_stronger_policy() {
    let base = policy_source("advisory", "positive");
    let empty_saved = ChelisSnapshot::from_files(Vec::new()).expect("empty head");
    let config = ChelisConfiguration::parse(
        b"schema = \"chelis-adapter-config/v1\"\n[[selector]]\nroot = \"chelis\"\ndialect = \"surf\"\n",
    ).expect("configuration");
    let absent = static_check(&empty_saved, &config).expect("absent policy");
    let removed = change_check(
        &base,
        &absent,
        &manifest(&[("coverage-policy", "CHELIS-BACKEND-POLICY", "removed")]),
    )
    .expect("removed policy");
    assert!(
        removed
            .diagnostics
            .iter()
            .any(|row| row.context.contains("policy-removed"))
    );
    let changed_scope = policy_source_with_scope("advisory", "positive", "other-selection");
    let changed = change_check(
        &base,
        &changed_scope,
        &manifest(&[("coverage-policy", "CHELIS-BACKEND-POLICY", "revised")]),
    )
    .expect("changed scope");
    assert!(
        changed
            .diagnostics
            .iter()
            .any(|row| row.context.contains("scope-changed"))
    );
    for head in [
        policy_source("advisory", "positive"),
        policy_source("blocking", "positive;negative"),
    ] {
        let outcome = change_check(
            &base,
            &head,
            &manifest(&[("coverage-policy", "CHELIS-BACKEND-POLICY", "revised")]),
        )
        .expect("equal or stronger policy");
        assert!(
            !codes(&outcome.diagnostics).contains(&"CHELIS-PROV-POLICY-WEAKENING"),
            "{:?}",
            outcome.diagnostics
        );
    }
}

#[test]
fn policy_weakening_protocol_fixture_cases_execute() {
    let cases = read_cases("change-manifests.toml");
    let weakening_cases = cases
        .iter()
        .filter(|case| {
            case_text(case, "operation") == "policy-ratchet"
                && case_text(case, "polarity") == "negative"
        })
        .collect::<Vec<_>>();
    assert_eq!(weakening_cases.len(), 4, "every protocol failure is frozen");
    let base = policy_source("blocking", "positive;negative");
    let head = policy_source("blocking", "positive");
    let steps = BTreeMap::from([
        ("change-weakening-missing-migration", "migration"),
        ("change-weakening-missing-approval", "approval"),
        ("change-weakening-missing-debt", "debt"),
        ("change-weakening-missing-waiver", "waiver"),
    ]);
    for case in &weakening_cases {
        let id = case_text(case, "id");
        let missing_step = steps.get(id).expect("frozen protocol case id");
        let mut protocol = vec![
            ("migration-record", "reviews/migration.md"),
            ("approval-record", "reviews/approval.md"),
            ("debt-id", "CHELIS-BACKEND-DEBT"),
            ("waiver-id", "CHELIS-BACKEND-WAIVER"),
        ];
        protocol.retain(|(key, _)| !key.starts_with(missing_step));
        let mut text =
            String::from("schema = \"chelis-change-manifest/v1\"\n\n[weakening-protocol]\n");
        for (key, value) in protocol {
            text.push_str(&format!("{key} = \"{value}\"\n"));
        }
        let manifest = ChangeManifest::parse(text.as_bytes()).expect("manifest");
        let outcome = change_check(&base, &head, &manifest).expect("change outcome");
        assert_eq!(
            outcome.exit,
            chelis_provenance::command::EXIT_CHANGE,
            "{id}"
        );
        let expected = case_diagnostic(case).expect("frozen diagnostic");
        assert!(
            outcome
                .diagnostics
                .iter()
                .any(|diagnostic| diagnostic.code == expected
                    && diagnostic
                        .context
                        .contains(&format!("protocol-step-missing:{missing_step}"))),
            "{id}: {:?}",
            outcome.diagnostics
        );
    }
    let complete = ChangeManifest::parse(
        b"schema = \"chelis-change-manifest/v1\"\n\n[weakening-protocol]\nmigration-record = \"reviews/migration.md\"\napproval-record = \"reviews/approval.md\"\ndebt-id = \"CHELIS-BACKEND-DEBT\"\nwaiver-id = \"CHELIS-BACKEND-WAIVER\"\n",
    )
    .expect("complete manifest");
    let outcome = change_check(&base, &head, &complete).expect("change outcome");
    assert_eq!(outcome.exit, EXIT_OK, "{:?}", outcome.diagnostics);
    assert!(
        !outcome.accepted_weakenings.is_empty(),
        "accepted weakening stays visible"
    );
    let mode_head = policy_source("advisory", "positive;negative");
    let outcome = change_check(
        &base,
        &mode_head,
        &ChangeManifest::parse(b"schema = \"chelis-change-manifest/v1\"\n")
            .expect("empty manifest"),
    )
    .expect("mode outcome");
    assert!(
        outcome.diagnostics.iter().any(|diagnostic| diagnostic.code
            == "CHELIS-PROV-POLICY-WEAKENING"
            && diagnostic.context.contains("blocking-to-advisory")),
        "{:?}",
        outcome.diagnostics
    );
}

#[test]
fn invalid_record_values_report_the_field_and_materialize_no_object() {
    let revision = atom_value("The subject MUST remain current.")
        .revision()
        .canonical_text();
    let authority = vec![
        ("id", String::from("INVALID-ATOM")),
        ("kind", String::from("behavioral")),
        ("scopes", String::from("chelis-core")),
        (
            "statement",
            String::from("The subject MUST remain current."),
        ),
    ];
    let surface = vec![
        ("id", String::from("INVALID-SURFACE")),
        ("member-key-schema", String::from("member/v1")),
        ("disposition-schema", String::from("disposition/v1")),
        ("generation", String::from("1")),
        ("members", String::from("eval;metal")),
        (
            "rows",
            String::from("eval:not-applicable;metal:not-applicable"),
        ),
    ];
    let carrier = vec![
        ("id", String::from("INVALID-CARRIER")),
        ("role", String::from("positive")),
        ("atoms", format!("INVALID-ATOM@{revision}")),
        (
            "item-digest",
            declaration_item_digest("def subject(x: f32) -> f32 = x"),
        ),
        ("oracle-id", String::from("INVALID-ORACLE")),
        ("oracle-digest", String::from(TOOL_DIGEST)),
        ("configuration-digest", String::from(ORACLE_CONFIGURATION)),
        ("scope-schema", String::from("scope/v1")),
        ("scope", String::from("subject")),
    ];
    let link = vec![
        ("id", String::from("INVALID-LINK")),
        ("atoms", format!("INVALID-ATOM@{revision}")),
        (
            "item-digest",
            declaration_item_digest("def subject(x: f32) -> f32 = x"),
        ),
        ("navigation", String::from("implements")),
    ];
    let policy = vec![
        ("id", String::from("INVALID-POLICY")),
        ("required-roles", String::from("positive")),
        ("enforcement", String::from("advisory")),
        ("selector-root", String::from("chelis")),
        ("selector-adapter", String::from("chelis-surf/v1")),
        ("selector-includes", String::from("**/*.ch")),
        ("scope-schema", String::from("scope/v1")),
        ("scope", String::from("subject")),
        ("ratchet", String::from("chelis-core")),
        ("approval-authority", String::from("chelis-core")),
        ("approval-record", String::from("reviews/policy.md")),
        ("approval-digest", String::from(TOOL_DIGEST)),
    ];
    let obligation = format!("INVALID-POLICY:freshness:atom:INVALID-ATOM@{TOOL_DIGEST}");
    let debt = vec![
        ("id", String::from("INVALID-DEBT")),
        ("obligations", obligation),
        ("objects", format!("atom:INVALID-ATOM@{TOOL_DIGEST}")),
        (
            "rationale",
            String::from("Current evidence requires repair"),
        ),
        ("owner", String::from("chelis-core")),
        ("remediation", String::from("Replace stale evidence")),
        ("discharge-schema", String::from("discharge/v1")),
        ("discharge", String::from("Evidence is current")),
    ];
    let oracle_extraction = chelis_provenance::records::extract(
        ChelisDialect::Surf,
        oracle_record(ChelisDialect::Surf).as_bytes(),
    )
    .expect("oracle extraction");
    let oracle = oracle_extraction.records[0]
        .fields
        .iter()
        .map(|(key, value)| (key.as_str(), value.clone()))
        .collect::<Vec<_>>();
    let derivation = vec![
        ("id", String::from("INVALID-DERIVATION")),
        ("relation", String::from("generated-from")),
        (
            "sources",
            format!("chelis/source.txt|text/plain|raw-bytes|{TOOL_DIGEST}"),
        ),
        (
            "targets",
            format!("chelis/target.txt|text/plain|raw-bytes|{TOOL_DIGEST}"),
        ),
        ("tool-id", String::from("chelis-generator")),
        ("tool-version", String::from("1.0.0")),
        ("tool-digest", String::from(TOOL_DIGEST)),
        (
            "tool-configuration-digest",
            String::from(TOOL_CONFIGURATION),
        ),
        ("configuration-digest", String::from(ORACLE_CONFIGURATION)),
        ("oracle-id", String::from("CHELIS-CONSISTENCY-ORACLE")),
        ("oracle-digest", oracle_digest()),
        ("scope-schema", String::from("scope/v1")),
        ("scope", String::from("subject")),
    ];
    let approval = vec![
        ("approval-authority", String::from("chelis-core")),
        ("approval-record", String::from("reviews/approval.md")),
        ("approval-digest", String::from(TOOL_DIGEST)),
    ];
    let mut waiver = debt.clone();
    waiver.extend(approval.clone());
    let mut formal_property = vec![
        ("id", String::from("INVALID-PROPERTY")),
        ("atoms", format!("INVALID-ATOM@{revision}")),
        ("property-schema", String::from("property/v1")),
        ("property", String::from("The subject is deterministic")),
        ("abstraction-schema", String::from("abstraction/v1")),
        ("abstraction", String::from("Pure subject")),
    ];
    formal_property.extend(approval.clone());
    let mut reaffirmation = vec![
        ("id", String::from("INVALID-REAFFIRMATION")),
        ("affected-kind", String::from("atom")),
        ("affected-id", String::from("INVALID-ATOM")),
        ("base-digest", String::from(TOOL_DIGEST)),
        ("head-digest", String::from(TOOL_CONFIGURATION)),
        (
            "statement",
            String::from("The review accepts the revised subject"),
        ),
    ];
    reaffirmation.extend(approval);
    let source_artifact = format!("chelis/source.txt|text/plain|raw-bytes|{TOOL_DIGEST}");
    let invalid_media = format!("chelis/source.txt||raw-bytes|{TOOL_DIGEST}");
    let missing_role = format!("INVALID-POLICY:required-role:atom:INVALID-ATOM@{TOOL_DIGEST}");
    let extra_role =
        format!("INVALID-POLICY:freshness:atom:INVALID-ATOM@{TOOL_DIGEST}!role=positive");
    let duplicate_atoms = format!("INVALID-ATOM@{revision};INVALID-ATOM@{revision}");
    let cases = [
        ("authority", &authority, "id", "invalid id", "invalid-id:id"),
        (
            "authority",
            &authority,
            "kind",
            "unknown",
            "invalid-atom-kind:unknown",
        ),
        (
            "authority",
            &authority,
            "scopes",
            "",
            "missing-field:scopes",
        ),
        (
            "authority",
            &authority,
            "scopes",
            "invalid scope",
            "invalid-scope:scopes",
        ),
        (
            "authority",
            &authority,
            "lifecycle",
            "retired",
            "invalid-lifecycle:retired",
        ),
        (
            "authority",
            &authority,
            "lifecycle",
            "ratified",
            "missing-field:approval-authority",
        ),
        (
            "authority",
            &authority,
            "statement",
            "The subject is current.",
            "invalid-atom",
        ),
        (
            "authority",
            &authority,
            "scopes",
            "chelis-core;chelis-core",
            "invalid-atom",
        ),
        (
            "carrier",
            &carrier,
            "role",
            "unknown",
            "invalid-role:unknown",
        ),
        (
            "carrier",
            &carrier,
            "atoms",
            "INVALID-ATOM",
            "invalid-atom-revision:INVALID-ATOM",
        ),
        (
            "carrier",
            &carrier,
            "atoms",
            "INVALID-ATOM@bad",
            "invalid-atom-revision:INVALID-ATOM@bad",
        ),
        (
            "carrier",
            &carrier,
            "atoms",
            "invalid id@bad",
            "invalid-atom-revision:invalid id@bad",
        ),
        ("carrier", &carrier, "corpora", "bad", "invalid-digest:bad"),
        (
            "carrier",
            &carrier,
            "scope-schema",
            "invalid schema",
            "invalid-schema:scope-schema",
        ),
        (
            "carrier",
            &carrier,
            "item-digest",
            "bad",
            "invalid-digest:item-digest",
        ),
        (
            "carrier",
            &carrier,
            "atoms",
            duplicate_atoms.as_str(),
            "invalid-carrier",
        ),
        (
            "implementation-link",
            &link,
            "atoms",
            duplicate_atoms.as_str(),
            "invalid-link",
        ),
        (
            "implementation-link",
            &link,
            "navigation",
            "unknown",
            "invalid-navigation:unknown",
        ),
        (
            "surface",
            &surface,
            "generation",
            "-1",
            "invalid-number:generation",
        ),
        (
            "surface",
            &surface,
            "member-key-schema",
            "invalid schema",
            "invalid-schema:member-key-schema",
        ),
        (
            "surface",
            &surface,
            "disposition-schema",
            "invalid schema",
            "invalid-schema:disposition-schema",
        ),
        (
            "surface",
            &surface,
            "rows",
            "eval:unknown",
            "invalid-row:eval:unknown",
        ),
        (
            "surface",
            &surface,
            "rows",
            "eval:normative:INVALID-ATOM",
            "invalid-atom-revision:INVALID-ATOM",
        ),
        (
            "surface",
            &surface,
            "rows",
            "eval:normative",
            "invalid-row:eval:normative",
        ),
        (
            "surface",
            &surface,
            "generation",
            "0",
            "invalid-surface-domain",
        ),
        (
            "surface",
            &surface,
            "members",
            "eval;eval",
            "invalid-surface-domain",
        ),
        (
            "surface",
            &surface,
            "rows",
            "eval:not-applicable;eval:not-applicable",
            "invalid-surface",
        ),
        (
            "policy",
            &policy,
            "enforcement",
            "unknown",
            "invalid-enforcement:unknown",
        ),
        (
            "policy",
            &policy,
            "selector-expectation",
            "unknown",
            "invalid-expectation:unknown",
        ),
        (
            "policy",
            &policy,
            "selector-root",
            "../escape",
            "invalid-path:selector-root",
        ),
        (
            "policy",
            &policy,
            "selector-adapter",
            "invalid schema",
            "invalid-schema:selector-adapter",
        ),
        (
            "policy",
            &policy,
            "approval-authority",
            "invalid scope",
            "invalid-scope:approval-authority",
        ),
        (
            "policy",
            &policy,
            "required-roles",
            "positive;positive",
            "invalid-policy-revision",
        ),
        (
            "policy",
            &policy,
            "selector-includes",
            "same;same",
            "invalid-selector",
        ),
        (
            "policy",
            &policy,
            "approval-record",
            "../escape",
            "invalid-path:approval-record",
        ),
        (
            "debt",
            &debt,
            "obligations",
            "INVALID-POLICY",
            "invalid-obligation:INVALID-POLICY",
        ),
        (
            "debt",
            &debt,
            "obligations",
            "INVALID-POLICY:freshness",
            "invalid-obligation:INVALID-POLICY:freshness",
        ),
        (
            "debt",
            &debt,
            "obligations",
            "INVALID-POLICY:unknown:atom:INVALID-ATOM@bad",
            "invalid-obligation-kind:unknown",
        ),
        (
            "debt",
            &debt,
            "obligations",
            "invalid policy:freshness:atom:INVALID-ATOM@xxh3-128:00000000000000000000000000000001",
            "invalid-obligation:invalid policy:freshness:atom:INVALID-ATOM@xxh3-128:00000000000000000000000000000001",
        ),
        (
            "debt",
            &debt,
            "objects",
            "INVALID-ATOM",
            "invalid-object-reference:INVALID-ATOM",
        ),
        (
            "debt",
            &debt,
            "objects",
            "atom:INVALID-ATOM",
            "invalid-object-reference:atom:INVALID-ATOM",
        ),
        (
            "debt",
            &debt,
            "objects",
            "unknown:INVALID-ATOM@bad",
            "invalid-object-kind:unknown",
        ),
        (
            "debt",
            &debt,
            "objects",
            "atom:INVALID-ATOM@bad",
            "invalid-object-reference:atom:INVALID-ATOM@bad",
        ),
        (
            "debt",
            &debt,
            "objects",
            "atom:invalid id@bad",
            "invalid-object-reference:atom:invalid id@bad",
        ),
        (
            "oracle",
            &oracle,
            "oracle-kind",
            "unknown",
            "invalid-oracle-kind:unknown",
        ),
        (
            "oracle",
            &oracle,
            "runner-id",
            "invalid scope",
            "invalid-scope:runner-id",
        ),
        (
            "oracle",
            &oracle,
            "runner-version",
            "",
            "missing-field:runner-version",
        ),
        (
            "oracle",
            &oracle,
            "runner-digest",
            "bad",
            "invalid-digest:runner-digest",
        ),
        (
            "oracle",
            &oracle,
            "selection-schema",
            "invalid schema",
            "invalid-schema:selection-schema",
        ),
        (
            "oracle",
            &oracle,
            "runner-version",
            "cafe\u{301}",
            "invalid-tool:runner",
        ),
        (
            "derivation",
            &derivation,
            "relation",
            "unknown",
            "invalid-relation:unknown",
        ),
        (
            "derivation",
            &derivation,
            "sources",
            "chelis/source.txt",
            "invalid-artifact:chelis/source.txt",
        ),
        (
            "derivation",
            &derivation,
            "sources",
            "../escape|text/plain|raw-bytes|bad",
            "invalid-artifact:../escape|text/plain|raw-bytes|bad",
        ),
        (
            "derivation",
            &derivation,
            "sources",
            "chelis/source.txt|text/plain|raw-bytes|bad",
            "invalid-artifact:chelis/source.txt|text/plain|raw-bytes|bad",
        ),
        (
            "derivation",
            &derivation,
            "tool-id",
            "invalid scope",
            "invalid-scope:tool-id",
        ),
        (
            "derivation",
            &derivation,
            "sources",
            invalid_media.as_str(),
            &format!("invalid-artifact:{invalid_media}"),
        ),
        (
            "derivation",
            &derivation,
            "targets",
            source_artifact.as_str(),
            "invalid-derivation",
        ),
        ("debt", &debt, "rationale", "cafe\u{301}", "invalid-debt"),
        (
            "debt",
            &debt,
            "obligations",
            missing_role.as_str(),
            &format!("invalid-obligation:{missing_role}"),
        ),
        (
            "debt",
            &debt,
            "obligations",
            extra_role.as_str(),
            &format!("invalid-obligation:{extra_role}"),
        ),
        (
            "waiver",
            &waiver,
            "rationale",
            "cafe\u{301}",
            "invalid-waiver",
        ),
        (
            "formal-property",
            &formal_property,
            "atoms",
            duplicate_atoms.as_str(),
            "invalid-formal-property",
        ),
        (
            "reaffirmation",
            &reaffirmation,
            "head-digest",
            TOOL_DIGEST,
            "invalid-reaffirmation",
        ),
        (
            "oracle",
            &oracle,
            "exclusions",
            "same;same",
            "invalid-oracle",
        ),
        (
            "debt",
            &debt,
            "obligations",
            "INVALID-POLICY:required-role:atom:INVALID-ATOM@bad!role=unknown",
            "invalid-role:unknown",
        ),
        (
            "debt",
            &debt,
            "obligations",
            "INVALID-POLICY:freshness:atom:INVALID-ATOM@bad",
            "invalid-object-reference:atom:INVALID-ATOM@bad",
        ),
        (
            "policy",
            &policy,
            "required-roles",
            "unknown",
            "invalid-role:unknown",
        ),
        (
            "waiver",
            &waiver,
            "objects",
            "atom:INVALID-ATOM@bad",
            "invalid-object-reference:atom:INVALID-ATOM@bad",
        ),
        (
            "reaffirmation",
            &reaffirmation,
            "affected-kind",
            "unknown",
            "invalid-object-kind:unknown",
        ),
    ];
    let configuration = ChelisConfiguration::parse(
        b"schema = \"chelis-adapter-config/v1\"\n[[selector]]\nroot = \"chelis\"\ndialect = \"surf\"\n",
    ).expect("configuration");
    for (kind, original, key, value, context) in cases {
        let mut fields = original.clone();
        if let Some(field) = fields.iter_mut().find(|(name, _)| *name == key) {
            field.1 = value.to_owned();
        } else {
            fields.push((key, value.to_owned()));
        }
        let text = record(
            ChelisDialect::Surf,
            kind,
            &fields,
            "def subject(x: f32) -> f32 = x",
        );
        let snapshot = ChelisSnapshot::from_files(vec![(
            String::from("chelis/invalid.ch"),
            ChelisDialect::Surf,
            text.as_bytes().to_vec(),
        )])
        .expect("snapshot");
        let outcome = static_check(&snapshot, &configuration).expect("static outcome");
        assert_eq!(outcome.exit, EXIT_STATIC, "{kind}:{key}");
        let diagnostic = outcome
            .diagnostics
            .iter()
            .find(|diagnostic| diagnostic.code == "CHELIS-PROV-MALFORMED-RECORD")
            .expect("invalid field diagnostic");
        assert_eq!(diagnostic.context, context, "{kind}:{key}");
        assert_eq!(diagnostic.path, "chelis/invalid.ch");
        assert_eq!(
            (diagnostic.start, diagnostic.end),
            (0, text.len() as u64 - 1)
        );
        assert_eq!(
            outcome.graph,
            buoy::core::SpecGraph::default(),
            "{kind}:{key}"
        );
    }
    for (kind, original) in [
        ("authority", &authority),
        ("carrier", &carrier),
        ("implementation-link", &link),
        ("surface", &surface),
        ("policy", &policy),
        ("debt", &debt),
        ("waiver", &waiver),
        ("formal-property", &formal_property),
        ("reaffirmation", &reaffirmation),
        ("derivation", &derivation),
        ("oracle", &oracle),
    ] {
        for (key, _) in original {
            if ["objects", "remediation"].contains(key) && kind == "waiver" {
                continue;
            }
            if *key == "objects" {
                continue;
            }
            let fields = original
                .iter()
                .filter(|(name, _)| name != key)
                .cloned()
                .collect::<Vec<_>>();
            let text = record(
                ChelisDialect::Surf,
                kind,
                &fields,
                "def subject(x: f32) -> f32 = x",
            );
            let saved = ChelisSnapshot::from_files(vec![(
                String::from("chelis/missing.ch"),
                ChelisDialect::Surf,
                text.into_bytes(),
            )])
            .expect("missing field snapshot");
            let outcome = static_check(&saved, &configuration).expect("missing field outcome");
            let diagnostic = outcome
                .diagnostics
                .iter()
                .find(|row| row.code == "CHELIS-PROV-MALFORMED-RECORD")
                .expect("missing field error");
            assert_eq!(
                diagnostic.context,
                format!("missing-field:{key}"),
                "{kind}:{key}"
            );
            assert_eq!(
                outcome.graph,
                buoy::core::SpecGraph::default(),
                "{kind}:{key}"
            );
        }
    }
    let prohibited = format!("eval:prohibited:INVALID-ATOM@{revision};metal:not-applicable");
    let deferred = format!("eval:deferred:INVALID-ATOM@{revision};metal:not-applicable");
    let obligation_values = [
        "authority",
        "surface-completeness",
        "surface-authority",
        "oracle-selection",
        "derivation-consistency",
        "proof",
        "change-impact",
    ]
    .map(|kind| format!("INVALID-POLICY:{kind}:atom:INVALID-ATOM@{TOOL_DIGEST}"));
    let obligation_text = obligation_values
        .iter()
        .map(String::as_str)
        .collect::<Vec<_>>();
    for (kind, original, key, values) in [
        ("authority", &authority, "kind", &["constraint"][..]),
        (
            "implementation-link",
            &link,
            "navigation",
            &["owns", "consumes", "configures"][..],
        ),
        (
            "surface",
            &surface,
            "rows",
            &[prohibited.as_str(), deferred.as_str()][..],
        ),
        (
            "policy",
            &policy,
            "selector-expectation",
            &["forbidden", "required"][..],
        ),
        (
            "derivation",
            &derivation,
            "relation",
            &["byte-equal", "structurally-equivalent", "host-consistency"][..],
        ),
        (
            "oracle",
            &oracle,
            "oracle-kind",
            &[
                "property",
                "exhaustive-model",
                "structural",
                "formal",
                "host",
            ][..],
        ),
        (
            "reaffirmation",
            &reaffirmation,
            "affected-kind",
            &[
                "decision-surface",
                "implementation-link",
                "coverage-policy",
                "derivation",
                "debt-record",
                "waiver",
                "oracle",
                "formal-property",
                "reaffirmation",
            ][..],
        ),
        ("debt", &debt, "obligations", obligation_text.as_slice()),
        (
            "formal-property",
            &formal_property,
            "property",
            &["The subject is total"][..],
        ),
    ] {
        let evaluate = |fields: &Vec<(&str, String)>| {
            let text = record(
                ChelisDialect::Surf,
                kind,
                fields,
                "def subject(x: f32) -> f32 = x",
            );
            let saved = ChelisSnapshot::from_files(vec![(
                String::from("chelis/enums.ch"),
                ChelisDialect::Surf,
                text.into_bytes(),
            )])
            .expect("enum snapshot");
            static_check(&saved, &configuration).expect("enum outcome")
        };
        let baseline = evaluate(original);
        for value in values {
            let mut fields = original.clone();
            if let Some(field) = fields.iter_mut().find(|(name, _)| *name == key) {
                field.1 = (*value).to_owned();
            } else {
                fields.push((key, (*value).to_owned()));
            }
            let outcome = evaluate(&fields);
            assert!(
                !codes(&outcome.diagnostics).contains(&"CHELIS-PROV-MALFORMED-RECORD"),
                "{kind}:{value}: {:?}",
                outcome.diagnostics
            );
            assert_ne!(outcome.graph, baseline.graph, "{kind}:{value}");
            let change = change_check(&baseline, &outcome, &manifest(&[])).expect("enum change");
            assert_eq!(
                codes(&change.diagnostics).contains(&"CHELIS-PROV-IMPACT-DISPOSITION-MISSING"),
                kind == "authority",
                "{kind}:{value}"
            );
        }
    }
}
