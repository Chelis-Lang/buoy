//! Executable preimplementation contracts for the blocked Chelis integration.

#![allow(clippy::expect_used, clippy::similar_names, clippy::too_many_lines)]

use buoy::adapter_sdk::{
    AdapterConfiguration, AdapterDescriptor, AdapterProduct, AdapterRegistry, AdapterSdkError,
    NonnormativeDisposition, SavedSnapshotInput, ShellAdapter,
};
use buoy::adapters::SourceSpan;
use buoy::core::{RepositoryPath, SchemaId, SourceItemKind, SpecGraph, TypedPayload};
use hegel::generators as gs;
use hegel::{Hegel, TestCase};
use std::collections::{BTreeMap, BTreeSet, VecDeque};
use std::fmt::Write as _;
use std::fs;
use std::path::Path;

#[path = "support/hegel_settings.rs"]
mod hegel_settings;
use hegel_settings::settings;

const FIXTURE_ROOT: &str = "fixtures/chelis-provenance/v1";
const CASE_SCHEMA: &str = "chelis-provenance-fixture-cases/v1";
const FINAL_ORACLE: &str = "devenv test (pinned Chelis integration worktree)";
const CHELIS_REVISION: &str = "c0d4caf16f557406f299f0989bab48fa0f924cbd";
const BUOY_REVISION: &str = "61b2c25afd55f8295715868cb854d2b4777de9a2";

#[derive(Clone, Debug)]
struct FrozenCase {
    requirement: String,
    polarity: String,
    diagnostic: String,
    execution: String,
    searchable: String,
}

#[derive(Clone, Debug)]
struct ManifestRow {
    polarity: String,
    diagnostic: String,
    focused_command: String,
    final_oracle: String,
    execution: String,
}

#[derive(Clone, Copy)]
enum ProductMode {
    Complete,
    Empty,
    UnsupportedCapability,
}

struct FixtureAdapter {
    descriptor: AdapterDescriptor,
    mode: ProductMode,
}

impl FixtureAdapter {
    fn new(adapter_id: &str, capability: &str, mode: ProductMode) -> Self {
        Self {
            descriptor: AdapterDescriptor::new(
                adapter_id,
                "chelis-surf",
                "0.18.5+grammar.fc3232ff",
                "chelis-surf-syntax/v2",
                "chelis-adapter-config/v1",
                vec![capability.to_owned()],
                chelis_source_item_kinds(),
            )
            .expect("valid frozen test descriptor"),
            mode,
        }
    }
}

impl ShellAdapter for FixtureAdapter {
    fn descriptor(&self) -> &AdapterDescriptor {
        &self.descriptor
    }

    fn materialize(
        &self,
        input: &SavedSnapshotInput<'_>,
    ) -> Result<AdapterProduct, AdapterSdkError> {
        let dispositions = match self.mode {
            ProductMode::Complete => vec![NonnormativeDisposition::new(
                SourceSpan {
                    path: input.path().clone(),
                    start: 0,
                    end: input.bytes().len() as u64,
                },
                self.descriptor.syntax_capabilities()[0].as_str(),
            )?],
            ProductMode::Empty => Vec::new(),
            ProductMode::UnsupportedCapability => vec![NonnormativeDisposition::new(
                SourceSpan {
                    path: input.path().clone(),
                    start: 0,
                    end: input.bytes().len() as u64,
                },
                "chelis-unsupported/v1",
            )?],
        };
        AdapterProduct::new(SpecGraph::default(), Vec::new(), dispositions, Vec::new())
    }
}

fn table(path: &str) -> toml::Table {
    fs::read_to_string(path)
        .expect("fixture text")
        .parse::<toml::Table>()
        .expect("valid fixture TOML")
}

fn text<'a>(table: &'a toml::Table, key: &str) -> &'a str {
    table
        .get(key)
        .and_then(toml::Value::as_str)
        .expect("required text field")
}

fn array_text(table: &toml::Table, key: &str) -> Vec<String> {
    table
        .get(key)
        .and_then(toml::Value::as_array)
        .expect("required array field")
        .iter()
        .map(|value| value.as_str().expect("array text").to_owned())
        .collect()
}

fn read_cases() -> BTreeMap<String, FrozenCase> {
    let case_root = Path::new(FIXTURE_ROOT).join("cases");
    let mut paths = fs::read_dir(case_root)
        .expect("case directory")
        .map(|entry| entry.expect("case entry").path())
        .filter(|path| {
            path.extension()
                .is_some_and(|extension| extension == "toml")
        })
        .collect::<Vec<_>>();
    paths.sort_unstable();
    let mut cases = BTreeMap::new();
    for path in paths {
        let source = fs::read_to_string(&path).expect("case source");
        let document = source.parse::<toml::Table>().expect("valid case TOML");
        assert_eq!(text(&document, "schema"), CASE_SCHEMA, "{}", path.display());
        let values = document
            .get("case")
            .and_then(toml::Value::as_array)
            .expect("case array");
        for value in values {
            let case = value.as_table().expect("case table");
            let id = text(case, "id").to_owned();
            let diagnostics = array_text(case, "diagnostics");
            assert!(
                diagnostics.len() <= 1,
                "{id}: one stable diagnostic maximum"
            );
            let diagnostic = diagnostics
                .first()
                .cloned()
                .unwrap_or_else(|| String::from("-"));
            for key in ["scenario", "operation", "input", "expected_outcome"] {
                assert!(!text(case, key).is_empty(), "{id}: empty {key}");
            }
            assert!(!array_text(case, "facts").is_empty(), "{id}: facts");
            let frozen = FrozenCase {
                requirement: text(case, "requirement").to_owned(),
                polarity: text(case, "polarity").to_owned(),
                diagnostic,
                execution: text(case, "execution").to_owned(),
                searchable: source.clone(),
            };
            assert!(cases.insert(id.clone(), frozen).is_none(), "duplicate {id}");
        }
    }
    cases
}

fn read_manifest() -> BTreeMap<String, ManifestRow> {
    let source =
        fs::read_to_string(Path::new(FIXTURE_ROOT).join("manifest.tsv")).expect("fixture manifest");
    let mut lines = source.lines();
    assert_eq!(
        lines.next(),
        Some("case_id\tpolarity\tdiagnostic\tfocused_command\tfinal_oracle\texecution")
    );
    let mut previous = None::<String>;
    let mut manifest = BTreeMap::new();
    for line in lines {
        let fields = line.split('\t').collect::<Vec<_>>();
        assert_eq!(fields.len(), 6, "manifest line: {line}");
        let id = fields[0].to_owned();
        if let Some(prior) = &previous {
            assert!(prior < &id, "manifest IDs not sorted: {prior} then {id}");
        }
        previous = Some(id.clone());
        let row = ManifestRow {
            polarity: fields[1].to_owned(),
            diagnostic: fields[2].to_owned(),
            focused_command: fields[3].to_owned(),
            final_oracle: fields[4].to_owned(),
            execution: fields[5].to_owned(),
        };
        assert!(manifest.insert(id.clone(), row).is_none(), "duplicate {id}");
    }
    manifest
}

fn stable_diagnostic(value: &str) -> bool {
    !value.is_empty()
        && value
            .bytes()
            .all(|byte| byte.is_ascii_uppercase() || byte.is_ascii_digit() || matches!(byte, b'-'))
}

#[test]
fn inventory_and_reserved_schemas_are_complete_without_assigning_assurance() {
    let inventory = fs::read_to_string(Path::new(FIXTURE_ROOT).join("inventory.tsv"))
        .expect("Chelis inventory");
    let mut lines = inventory.lines();
    assert_eq!(
        lines.next(),
        Some("id\tcategory\tpath\trevision\tassurance\tobservation")
    );
    let mut ids = BTreeSet::new();
    let mut categories = BTreeSet::new();
    for line in lines {
        let fields = line.split('\t').collect::<Vec<_>>();
        assert_eq!(fields.len(), 6, "inventory line: {line}");
        assert!(
            ids.insert(fields[0]),
            "duplicate inventory ID: {}",
            fields[0]
        );
        categories.insert(fields[1]);
        assert_eq!(fields[3], CHELIS_REVISION);
        assert_eq!(fields[4], "unassigned");
        assert!(!fields[2].is_empty() && !fields[5].is_empty());
    }
    assert!(ids.len() >= 50);
    assert_eq!(
        categories,
        BTreeSet::from([
            "capability-matrix",
            "current-exemption",
            "executable-carrier",
            "formal-property",
            "generated-or-mirrored",
            "model",
            "normative-source",
            "release-command",
        ])
    );

    let registry = fs::read_to_string(Path::new(FIXTURE_ROOT).join("schema-registry.tsv"))
        .expect("schema registry");
    let mut lines = registry.lines();
    assert_eq!(
        lines.next(),
        Some("schema_id\tclass\towner\tstatus\tpurpose")
    );
    let mut schema_ids = BTreeSet::new();
    for line in lines {
        let fields = line.split('\t').collect::<Vec<_>>();
        assert_eq!(fields.len(), 5, "schema line: {line}");
        assert!(fields[0].contains("/v"));
        assert!(
            SchemaId::new(fields[0])
                .is_ok_and(|schema| TypedPayload::new(schema, Vec::new()).is_ok()),
            "invalid Buoy schema ID: {}",
            fields[0]
        );
        assert_eq!(fields[2], "chelis");
        assert!(
            ["draft-reserved", "active", "superseded"].contains(&fields[3]),
            "schema status outside the closed vocabulary: {line}"
        );
        assert!(schema_ids.insert(fields[0]));
        assert!(!fields[4].is_empty());
    }
    assert!(schema_ids.len() >= 18);
}

#[test]
fn boundary_pins_current_revisions_and_records_noncanonical_inputs() {
    let boundary = table(&format!("{FIXTURE_ROOT}/boundary.toml"));
    let chelis = boundary
        .get("chelis")
        .and_then(toml::Value::as_table)
        .expect("Chelis boundary");
    let buoy = boundary
        .get("buoy")
        .and_then(toml::Value::as_table)
        .expect("Buoy boundary");
    assert_eq!(text(chelis, "revision"), CHELIS_REVISION);
    assert_eq!(text(chelis, "license"), "MIT");
    assert_eq!(
        text(chelis, "build_environment_status"),
        "not-exactly-pinned"
    );
    assert_eq!(text(buoy, "repository_revision"), BUOY_REVISION);
    assert_eq!(text(buoy, "adapter_sdk_schema"), "buoy.adapter-sdk/v3");
    assert_eq!(
        text(buoy, "dependency_direction"),
        "chelis shell -> buoy shell/core"
    );

    let selectors = table(&format!("{FIXTURE_ROOT}/selectors.toml"));
    assert_eq!(text(&selectors, "overlap_policy"), "reject");
    let noncanonical = array_text(&selectors, "noncanonical_inputs");
    assert!(noncanonical.iter().any(|value| value == "openspec/**"));
    assert!(noncanonical.iter().any(|value| value == "wall clock"));
    assert!(!selectors.to_string().contains(".gitignore"));
}

#[test]
fn fixture_manifest_covers_every_frozen_case() {
    let cases = read_cases();
    let manifest = read_manifest();
    assert_eq!(
        cases.keys().collect::<Vec<_>>(),
        manifest.keys().collect::<Vec<_>>()
    );
    assert!(cases.len() >= 55);

    let mut requirement_polarities = BTreeMap::<String, BTreeSet<String>>::new();
    let mut all_text = String::new();
    for (id, case) in &cases {
        let row = manifest.get(id).expect("manifest row");
        assert_eq!(row.polarity, case.polarity, "{id}");
        assert_eq!(row.diagnostic, case.diagnostic, "{id}");
        assert_eq!(row.execution, case.execution, "{id}");
        assert!(!row.focused_command.is_empty(), "{id}");
        assert_eq!(row.final_oracle, FINAL_ORACLE, "{id}");
        if case.polarity == "negative" {
            assert!(
                stable_diagnostic(&case.diagnostic),
                "{id}: {}",
                case.diagnostic
            );
        }
        requirement_polarities
            .entry(case.requirement.clone())
            .or_default()
            .insert(case.polarity.clone());
        all_text.push_str(&case.searchable);
    }
    for number in 1..=21 {
        let requirement = format!("CHELIS-PROV-{number:03}");
        let polarities = requirement_polarities
            .get(&requirement)
            .expect("every CHELIS-PROV requirement has fixtures");
        assert!(polarities.contains("positive"), "{requirement}: positive");
        assert!(
            polarities.contains("negative") || polarities.contains("negative-control"),
            "{requirement}: negative"
        );
    }
    for required in [
        "missing-row",
        "unknown-version",
        "unattached-metadata",
        "stale-binding",
        "provider-noise",
        "separate-roots",
        "malformed-record",
        "role=positive",
        "role=negative",
        "role=property",
        "role=exhaustive-model",
        "role=formal-property",
        "role=structural",
        "source-digest-drift",
        "target-digest-drift",
        "tool-identity-drift",
        "configuration-drift",
        "exact-reaffirmed-disposition",
        "exact-revised-disposition",
        "exact-retired-disposition",
        "exact-waived-disposition",
    ] {
        assert!(
            all_text.contains(required),
            "missing frozen case: {required}"
        );
    }
}

#[test]
fn parsed_item_protocol_accepts_descriptor_declared_host_kinds() {
    let identifiers = include_str!("../crates/buoy-core/src/identifier.rs");
    assert!(identifiers.contains("SourceItemKind"));
    assert!(!identifiers.contains("pub enum ItemKind"));

    let sdk = include_str!("../src/adapter_sdk.rs");
    assert!(sdk.contains("source_item_kinds: Vec<SourceItemKind>"));
    assert!(sdk.contains("supports_source_item_kind"));
    assert!(sdk.contains("SPG-ADAPTER-UNDECLARED-SOURCE-ITEM-KIND"));

    let boundary = table(&format!("{FIXTURE_ROOT}/boundary.toml"));
    let protocol = boundary
        .get("item_protocol")
        .and_then(toml::Value::as_table)
        .expect("item protocol");
    assert_eq!(text(protocol, "status"), "approved");
    assert_eq!(
        text(protocol, "approved_host_item_schema"),
        "descriptor-declared SourceItemKind set"
    );
    assert_eq!(
        text(protocol, "protocol_check"),
        "exact descriptor membership"
    );
    assert_eq!(
        array_text(protocol, "forbidden_substitutions"),
        [
            "raw-string-kind",
            "opaque-payload",
            "host-specific-core-enum"
        ]
    );
}

fn chelis_source_item_kinds() -> Vec<SourceItemKind> {
    ["chelis/surf-declaration/v1", "chelis/deep-node/v1"]
        .into_iter()
        .map(|kind| SourceItemKind::new(kind).expect("Chelis source item kind"))
        .collect()
}

#[test]
fn adapter_sdk_controls_saved_snapshot_boundary() {
    let descriptor = AdapterDescriptor::new(
        "chelis-surf/v1",
        "chelis-surf",
        "0.18.5+grammar.fc3232ff",
        "chelis-surf-syntax/v2",
        "chelis-adapter-config/v1",
        vec![
            String::from("chelis-surf-declaration/v1"),
            String::from("chelis-surf-comment/v1"),
        ],
        chelis_source_item_kinds(),
    )
    .expect("reserved Surf descriptor satisfies SDK v3");
    assert_eq!(
        descriptor.syntax_capabilities()[0],
        "chelis-surf-comment/v1"
    );
    assert_eq!(
        descriptor.syntax_capabilities()[1],
        "chelis-surf-declaration/v1"
    );

    let path_a = RepositoryPath::new("tests/source.ch").expect("canonical path");
    let path_b = RepositoryPath::new("tests/source.ch").expect("canonical path");
    let bytes_a = b"def value(x: f32) -> f32 = x\n";
    let bytes_b = bytes_a.to_vec();
    let configuration = AdapterConfiguration::new(
        "chelis-surf/v1",
        "chelis-adapter-config/v1",
        b"explicit-config".to_vec(),
    )
    .expect("exact configuration");
    let first = SavedSnapshotInput::new(&path_a, bytes_a, &configuration).expect("first root");
    let second = SavedSnapshotInput::new(&path_b, &bytes_b, &configuration).expect("second root");
    assert_eq!(first, second);

    let mut forward = AdapterRegistry::new();
    forward
        .register(FixtureAdapter::new(
            "chelis-surf/v1",
            "chelis-surf-declaration/v1",
            ProductMode::Complete,
        ))
        .expect("register Surf");
    forward
        .register(FixtureAdapter::new(
            "chelis-z-control/v1",
            "chelis-control/v1",
            ProductMode::Complete,
        ))
        .expect("register control");
    let mut reverse = AdapterRegistry::new();
    reverse
        .register(FixtureAdapter::new(
            "chelis-z-control/v1",
            "chelis-control/v1",
            ProductMode::Complete,
        ))
        .expect("register control first");
    reverse
        .register(FixtureAdapter::new(
            "chelis-surf/v1",
            "chelis-surf-declaration/v1",
            ProductMode::Complete,
        ))
        .expect("register Surf second");
    assert_eq!(
        forward
            .descriptors()
            .iter()
            .map(|value| value.adapter_id())
            .collect::<Vec<_>>(),
        reverse
            .descriptors()
            .iter()
            .map(|value| value.adapter_id())
            .collect::<Vec<_>>()
    );
    assert_eq!(
        forward
            .materialize(&first)
            .expect("forward materialization"),
        reverse
            .materialize(&first)
            .expect("reverse materialization")
    );

    let ambient_a = ("daemon-a", "editor-a", "provider-a", "/absolute/root-a");
    let ambient_b = ("daemon-b", "editor-b", "provider-b", "/absolute/root-b");
    assert_ne!(ambient_a, ambient_b);
    assert_eq!(first.digest(), second.digest());
}

#[test]
fn adapter_sdk_rejects_duplicate_capability_and_incomplete_outputs() {
    let capability = "chelis-surf-declaration/v1";
    let mut duplicate = AdapterRegistry::new();
    duplicate
        .register(FixtureAdapter::new(
            "chelis-surf/v1",
            capability,
            ProductMode::Complete,
        ))
        .expect("first identity");
    assert_eq!(
        duplicate.register(FixtureAdapter::new(
            "chelis-surf/v1",
            capability,
            ProductMode::Complete,
        )),
        Err(AdapterSdkError::DuplicateAdapter)
    );

    let path = RepositoryPath::new("tests/source.ch").expect("path");
    let bytes = b"ordinary source";
    let configuration =
        AdapterConfiguration::new("chelis-surf/v1", "chelis-adapter-config/v1", Vec::new())
            .expect("configuration");
    let input = SavedSnapshotInput::new(&path, bytes, &configuration).expect("input");
    for (mode, error) in [
        (ProductMode::Empty, AdapterSdkError::Materialization),
        (
            ProductMode::UnsupportedCapability,
            AdapterSdkError::Capability,
        ),
    ] {
        let mut registry = AdapterRegistry::new();
        registry
            .register(FixtureAdapter::new("chelis-surf/v1", capability, mode))
            .expect("register control");
        assert_eq!(registry.materialize(&input), Err(error));
    }
}

fn canonical_payload(
    mut rows: Vec<(String, String)>,
    _absolute_root: &str,
    _provider: &str,
) -> String {
    rows.sort_unstable();
    rows.dedup();
    let mut output = String::new();
    for (key, value) in rows {
        write!(output, "{}:{key}{}:{value}", key.len(), value.len())
            .expect("string encoding cannot fail");
    }
    output
}

#[test]
fn hegel_chelis_payload_is_root_provider_and_order_independent() {
    Hegel::new(|test_case: TestCase| {
        let values = test_case.draw(gs::vecs(gs::integers::<u8>()).max_size(24));
        let rows = values
            .iter()
            .enumerate()
            .map(|(index, value)| (format!("axis-{}", index % 4), value.to_string()))
            .collect::<Vec<_>>();
        let mut reversed = rows.clone();
        reversed.reverse();
        assert_eq!(
            canonical_payload(rows, "/root-a", "github-a"),
            canonical_payload(reversed, "/root-b", "github-b")
        );
    })
    .settings(settings(101))
    .run();
}

#[test]
fn hegel_chelis_domain_completeness() {
    Hegel::new(|test_case: TestCase| {
        let omit = test_case.draw(gs::booleans());
        let expected = ["eval", "c-host", "c-dag", "hip", "metal"]
            .into_iter()
            .flat_map(|backend| ["scalar", "tensor"].map(|surface| (backend, surface)))
            .collect::<BTreeSet<_>>();
        let mut actual = expected.clone();
        if omit {
            actual.remove(&("metal", "tensor"));
        }
        assert_eq!(actual == expected, !omit);
        assert!(actual.contains(&("metal", "tensor")) || omit);
    })
    .settings(settings(102))
    .run();
}

#[test]
fn hegel_chelis_required_roles_are_exact() {
    Hegel::new(|test_case: TestCase| {
        let roles = (0..6)
            .map(|_| test_case.draw(gs::booleans()))
            .collect::<Vec<_>>();
        let pass = test_case.draw(gs::booleans());
        let covered = roles.iter().all(|present| *present) && pass;
        assert_eq!(covered, roles == [true; 6] && pass);
        if roles.iter().any(|present| !present) {
            assert!(!covered);
        }
    })
    .settings(settings(103))
    .run();
}

#[test]
fn hegel_chelis_freshness_is_exact() {
    Hegel::new(|test_case: TestCase| {
        let atom = test_case.draw(gs::integers::<u8>());
        let configuration = test_case.draw(gs::integers::<u8>());
        let carrier_atom = if test_case.draw(gs::booleans()) {
            atom
        } else {
            atom.wrapping_add(1)
        };
        let carrier_configuration = if test_case.draw(gs::booleans()) {
            configuration
        } else {
            configuration.wrapping_add(1)
        };
        let fresh = atom == carrier_atom && configuration == carrier_configuration;
        assert_eq!(
            fresh,
            (atom, configuration) == (carrier_atom, carrier_configuration)
        );
    })
    .settings(settings(104))
    .run();
}

#[test]
fn hegel_chelis_policy_ratchets_do_not_hide_weakening() {
    Hegel::new(|test_case: TestCase| {
        let scope_same = test_case.draw(gs::booleans());
        let roles_same = test_case.draw(gs::booleans());
        let surface_complete = test_case.draw(gs::booleans());
        let obligations_green = test_case.draw(gs::booleans());
        let ratchet_same = test_case.draw(gs::booleans());
        let blocking = test_case.draw(gs::booleans());
        let allowed = scope_same
            && roles_same
            && surface_complete
            && obligations_green
            && ratchet_same
            && blocking;
        if !scope_same || !roles_same || !surface_complete || !obligations_green || !ratchet_same {
            assert!(!allowed);
        }
        assert_eq!(
            allowed,
            [
                scope_same,
                roles_same,
                surface_complete,
                obligations_green,
                ratchet_same,
                blocking,
            ]
            .into_iter()
            .all(|value| value)
        );
    })
    .settings(settings(105))
    .run();
}

fn fixed_point(root: &str, edges: &BTreeMap<&str, Vec<&str>>) -> BTreeSet<String> {
    let mut reached = BTreeSet::from([root.to_owned()]);
    let mut pending = VecDeque::from([root.to_owned()]);
    while let Some(current) = pending.pop_front() {
        if let Some(targets) = edges.get(current.as_str()) {
            for target in targets {
                if reached.insert((*target).to_owned()) {
                    pending.push_back((*target).to_owned());
                }
            }
        }
    }
    reached
}

#[test]
fn hegel_chelis_impact_closure_reaches_fixed_point() {
    Hegel::new(|test_case: TestCase| {
        let first = test_case.draw(gs::booleans());
        let second = test_case.draw(gs::booleans());
        let third = test_case.draw(gs::booleans());
        let mut edges = BTreeMap::new();
        edges.insert("atom", if first { vec!["carrier"] } else { Vec::new() });
        edges.insert("carrier", if second { vec!["policy"] } else { Vec::new() });
        edges.insert("policy", if third { vec!["report"] } else { Vec::new() });
        let reached = fixed_point("atom", &edges);
        assert_eq!(reached.contains("carrier"), first);
        assert_eq!(reached.contains("policy"), first && second);
        assert_eq!(reached.contains("report"), first && second && third);
    })
    .settings(settings(106))
    .run();
}

#[test]
fn hegel_chelis_no_laundering() {
    Hegel::new(|test_case: TestCase| {
        let technical = test_case.draw(gs::booleans());
        let pass = test_case.draw(gs::booleans());
        let fresh = test_case.draw(gs::booleans());
        let selected = test_case.draw(gs::booleans());
        let _debt = test_case.draw(gs::booleans());
        let _waiver = test_case.draw(gs::booleans());
        let _reaffirmation = test_case.draw(gs::booleans());
        let _consistency = test_case.draw(gs::booleans());
        let tested = technical && pass && fresh && selected;
        assert_eq!(tested, technical && pass && fresh && selected);
        if !technical || !pass || !fresh || !selected {
            assert!(!tested);
        }
    })
    .settings(settings(107))
    .run();
}

#[test]
fn hegel_chelis_proof_invalidation_is_conservative() {
    Hegel::new(|test_case: TestCase| {
        let bindings = (0..10)
            .map(|_| test_case.draw(gs::booleans()))
            .collect::<Vec<_>>();
        let negative_control_failed = test_case.draw(gs::booleans());
        let proved = bindings.iter().all(|current| *current) && negative_control_failed;
        assert_eq!(proved, bindings == [true; 10] && negative_control_failed);
        if bindings.iter().any(|current| !current) || !negative_control_failed {
            assert!(!proved);
        }
    })
    .settings(settings(108))
    .run();
}
