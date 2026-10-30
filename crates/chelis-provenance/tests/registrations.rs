//! Fail-closed validation of the Chelis assurance registration pack.

#![allow(clippy::expect_used, clippy::panic, clippy::too_many_lines)]

use buoy::core::CanonicalValue;
use chelis_provenance::advisory::advisory_report;
use chelis_provenance::command::static_check;
use chelis_provenance::records::ChelisDialect;
use chelis_provenance::snapshot::{ChelisConfiguration, ChelisSnapshot};
use std::collections::BTreeSet;
use std::fmt::Write as _;
use std::fs;
use std::path::Path;

const FIXTURE_ROOT: &str = "../../fixtures/chelis-provenance/v1";
const CHELIS_REVISION: &str = "c0d4caf16f557406f299f0989bab48fa0f924cbd";

fn table(name: &str) -> toml::Table {
    let path = Path::new(FIXTURE_ROOT).join(name);
    fs::read_to_string(&path)
        .expect("registration file")
        .parse::<toml::Table>()
        .expect("registration TOML")
}

fn rows<'a>(document: &'a toml::Table, key: &str) -> Vec<&'a toml::Table> {
    document
        .get(key)
        .and_then(toml::Value::as_array)
        .expect("registration rows")
        .iter()
        .map(|value| value.as_table().expect("registration row"))
        .collect()
}

fn text<'a>(row: &'a toml::Table, key: &str) -> &'a str {
    row.get(key)
        .and_then(toml::Value::as_str)
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| panic!("missing registration field {key}"))
}

fn inventory_ids(category: &str) -> BTreeSet<String> {
    let inventory =
        fs::read_to_string(Path::new(FIXTURE_ROOT).join("inventory.tsv")).expect("inventory");
    inventory
        .lines()
        .skip(1)
        .filter_map(|line| {
            let fields = line.split('\t').collect::<Vec<_>>();
            (fields[1] == category).then(|| fields[0].to_owned())
        })
        .collect()
}

#[test]
fn every_current_chelis_carrier_is_registered_exactly() {
    let document = table("registrations/carriers.toml");
    let carriers = rows(&document, "carrier");
    let registered = carriers
        .iter()
        .map(|row| text(row, "inventory_id").to_owned())
        .collect::<BTreeSet<_>>();
    assert_eq!(
        registered,
        inventory_ids("executable-carrier"),
        "carrier registrations must cover the executable-carrier inventory exactly"
    );
    let roles = BTreeSet::from([
        "positive",
        "negative",
        "property",
        "exhaustive-model",
        "formal-property",
        "structural",
    ]);
    let verdicts = BTreeSet::from(["not-run", "pass", "fail", "error", "timeout"]);
    for row in &carriers {
        let id = text(row, "id");
        assert!(id.starts_with("CHELIS-CARRIER-"), "{id}");
        assert!(roles.contains(text(row, "role")), "{id}: explicit role");
        assert!(
            verdicts.contains(text(row, "verdict")),
            "{id}: retained five-state verdict"
        );
        assert_eq!(
            text(row, "configuration"),
            format!("chelis@{CHELIS_REVISION}"),
            "{id}: exact configuration"
        );
        for key in ["item", "atoms", "oracle", "corpus", "scope"] {
            assert!(!text(row, key).is_empty(), "{id}: {key}");
        }
        for key in ["assumptions", "exclusions"] {
            let values = row
                .get(key)
                .and_then(toml::Value::as_array)
                .expect("registration list");
            assert!(!values.is_empty(), "{id}: {key}");
        }
    }
}

#[test]
fn every_intentional_derivation_is_registered_exactly() {
    let document = table("registrations/derivations.toml");
    let derivations = rows(&document, "derivation");
    let registered = derivations
        .iter()
        .map(|row| text(row, "inventory_id").to_owned())
        .collect::<BTreeSet<_>>();
    assert_eq!(
        registered,
        inventory_ids("generated-or-mirrored"),
        "derivation registrations must cover the generated-or-mirrored inventory exactly"
    );
    let relations = BTreeSet::from([
        "byte-equal",
        "generated-from",
        "structurally-equivalent",
        "host-consistency",
    ]);
    for row in &derivations {
        let id = text(row, "id");
        assert!(relations.contains(text(row, "relation")), "{id}");
        for key in [
            "sources",
            "targets",
            "tool",
            "configuration",
            "consistency_oracle",
            "scope",
        ] {
            assert!(!text(row, key).is_empty(), "{id}: {key}");
        }
        let exclusions = row
            .get("exclusions")
            .and_then(toml::Value::as_array)
            .expect("exclusions");
        assert!(!exclusions.is_empty(), "{id}: exclusions");
    }
    assert!(!text(&document, "untracked_duplicates").is_empty());
}

#[test]
fn exhaustive_reports_name_exact_models_digests_bounds_and_results() {
    let document = table("registrations/exhaustive-reports.toml");
    let reports = rows(&document, "report");
    assert_eq!(reports.len(), 6);
    let mut state_total = 0_i64;
    for row in &reports {
        let model_path = text(row, "model");
        let source = fs::read_to_string(Path::new("../..").join(model_path)).expect("model file");
        let model = source.parse::<toml::Table>().expect("model TOML");
        assert_eq!(
            model.get("id").and_then(toml::Value::as_str),
            Some(text(row, "id")),
            "{model_path}: exact model ID"
        );
        assert_eq!(
            model.get("kind").and_then(toml::Value::as_str),
            Some(text(row, "kind")),
            "{model_path}: exact model kind"
        );
        let states = model
            .get("expected_state_count")
            .and_then(toml::Value::as_integer)
            .expect("state count");
        assert_eq!(
            row.get("states").and_then(toml::Value::as_integer),
            Some(states),
            "{model_path}: exact bound"
        );
        state_total += states;
        let dimensions = i64::try_from(
            model
                .get("dimensions")
                .and_then(toml::Value::as_array)
                .expect("dimensions")
                .len(),
        )
        .expect("dimension count");
        assert_eq!(
            row.get("dimensions").and_then(toml::Value::as_integer),
            Some(dimensions),
            "{model_path}: exact state/event definitions"
        );
        let invariants = i64::try_from(
            model
                .get("invariants")
                .and_then(toml::Value::as_array)
                .expect("invariants")
                .len(),
        )
        .expect("invariant count");
        assert_eq!(
            row.get("invariants").and_then(toml::Value::as_integer),
            Some(invariants),
            "{model_path}: exact invariants"
        );
        assert_eq!(
            row.get("fixed_point").and_then(toml::Value::as_bool),
            Some(true),
            "{model_path}: fixed-point result"
        );
        let digest = buoy::core::semantic_digest(
            "chelis-raw-artifact",
            vec![(1, CanonicalValue::Bytes(source.into_bytes()))],
        )
        .expect("model digest")
        .canonical_text();
        assert_eq!(
            text(row, "digest"),
            digest,
            "{model_path}: exact model digest"
        );
    }
    assert_eq!(
        document
            .get("state_total")
            .and_then(toml::Value::as_integer),
        Some(state_total),
        "exact enumerated state total"
    );
}

#[test]
fn proof_bindings_stay_conservative_without_approved_correspondence() {
    let document = table("registrations/proof-bindings.toml");
    let spike = document
        .get("spike-review")
        .and_then(toml::Value::as_table)
        .expect("spike review");
    assert_eq!(
        text(spike, "decision"),
        "no new proof representation accepted"
    );
    assert!(text(spike, "verus_spike").contains("source_item_kind"));
    let gate = document
        .get("acceptance-gate")
        .and_then(toml::Value::as_table)
        .expect("acceptance gate");
    let required = gate
        .get("required_bindings")
        .and_then(toml::Value::as_array)
        .expect("required bindings");
    assert!(required.len() >= 9, "complete acceptance bindings");
    let accepted = gate
        .get("accepted_records")
        .and_then(toml::Value::as_array)
        .expect("accepted records");
    assert!(
        accepted.is_empty(),
        "no Chelis proof record may be accepted without every binding"
    );
    let exclusions = gate
        .get("downstream_exclusions")
        .and_then(toml::Value::as_array)
        .expect("downstream exclusions");
    assert!(exclusions.len() >= 4, "backend exclusions stay recorded");
}

#[test]
fn promotion_review_keeps_advisory_policy_with_visible_gaps() {
    let document = table("registrations/promotion-review.toml");
    assert_eq!(text(&document, "enforcement"), "advisory");
    assert_eq!(text(&document, "promotion"), "not-approved");
    assert_eq!(
        text(&document, "ratchet_identity"),
        "chelis-provenance-initial"
    );
    let gaps = document
        .get("unsatisfied_obligations")
        .and_then(toml::Value::as_array)
        .expect("unsatisfied obligations");
    assert!(
        !gaps.is_empty(),
        "advisory retention requires visible unsatisfied obligations"
    );
    let waiver = document
        .get("waiver-policy")
        .and_then(toml::Value::as_table)
        .expect("waiver policy");
    assert_eq!(text(waiver, "synthesized_satisfaction"), "forbidden");
}

#[test]
fn adversarial_review_covers_every_requirement_with_resolved_findings() {
    let document = table("registrations/adversarial-review.toml");
    assert_eq!(
        document
            .get("unresolved_findings")
            .and_then(toml::Value::as_integer),
        Some(0),
        "every assurance-inflation finding must be resolved"
    );
    let reviews = rows(&document, "requirement");
    let reviewed = reviews
        .iter()
        .map(|row| text(row, "id").to_owned())
        .collect::<BTreeSet<_>>();
    let expected = (1..=21)
        .map(|number| format!("CHELIS-PROV-{number:03}"))
        .collect::<BTreeSet<_>>();
    assert_eq!(reviewed, expected, "every requirement is reviewed");
    for row in &reviews {
        let id = text(row, "id");
        assert!(!text(row, "review").is_empty(), "{id}");
        assert!(!text(row, "finding").is_empty(), "{id}");
        assert!(
            text(row, "resolution").starts_with("resolved"),
            "{id}: unresolved finding"
        );
    }
}

#[test]
fn advisory_report_materializes_every_visible_gap_class() {
    let uncovered = "-- chelis:provenance/v1 authority\n-- id = CHELIS-UNCOVERED\n-- kind = behavioral\n-- scopes = chelis-core\n-- statement = Uncovered claims MUST stay visible in advisory output.\ndef uncovered(x: f32) -> f32 = x\n";
    let copy = "def duplicate(x: f32) -> f32 = x\n";
    let snapshot = ChelisSnapshot::from_files(vec![
        (
            String::from("chelis/a.ch"),
            ChelisDialect::Surf,
            uncovered.as_bytes().to_vec(),
        ),
        (
            String::from("chelis/copy-one.ch"),
            ChelisDialect::Surf,
            copy.as_bytes().to_vec(),
        ),
        (
            String::from("chelis/copy-two.ch"),
            ChelisDialect::Surf,
            copy.as_bytes().to_vec(),
        ),
        (
            String::from("chelis/loose.ch"),
            ChelisDialect::Surf,
            b"-- the parser MUST accept unatomized text visibly\ndef loose(x: f32) -> f32 = x\n"
                .to_vec(),
        ),
    ])
    .expect("snapshot");
    let configuration = ChelisConfiguration::parse(
        b"schema = \"chelis-adapter-config/v1\"\n\n[[selector]]\nroot = \"chelis\"\ndialect = \"surf\"\n",
    )
    .expect("configuration");
    let outcome = static_check(&snapshot, &configuration).expect("static outcome");
    let report = advisory_report(&snapshot, &outcome).expect("advisory report");
    assert!(
        report.contains("\"uncovered_atoms\":[\"CHELIS-UNCOVERED\"]"),
        "{report}"
    );
    assert!(
        report.contains("chelis/copy-one.ch|chelis/copy-two.ch"),
        "{report}"
    );
    assert!(report.contains("chelis/loose.ch:1"), "{report}");
    assert!(report.contains("\"carrier_counts\":{}"), "{report}");
    let identical = advisory_report(&snapshot, &outcome).expect("second advisory report");
    assert_eq!(report, identical, "advisory bytes stay deterministic");

    let atom = &outcome.graph.atoms[0];
    let revision = atom.revision().canonical_text();
    let digest = atom.meta().digest().canonical_text();
    let mut rich_source = uncovered.to_owned();
    for (index, role) in [
        "positive",
        "negative",
        "property",
        "exhaustive-model",
        "formal-property",
        "structural",
    ]
    .iter()
    .enumerate()
    {
        let declaration = format!("def evidence_{index}(x: f32) -> f32 = x");
        let item = chelis_provenance::command::artifact_digest(declaration.as_bytes())
            .expect("item digest")
            .canonical_text();
        write!(
            rich_source,
            "-- chelis:provenance/v1 carrier\n-- id = GAP-CARRIER-{index}\n-- role = {role}\n-- atoms = CHELIS-UNCOVERED@{revision}\n-- item-digest = {item}\n-- oracle-id = GAP-ORACLE\n-- oracle-digest = {digest}\n-- configuration-digest = {digest}\n-- scope-schema = gap-scope/v1\n-- scope = evidence\n{declaration}\n"
        )
        .expect("carrier source");
    }
    write!(
        rich_source,
        "-- chelis:provenance/v1 surface\n-- id = GAP-SURFACE\n-- member-key-schema = member/v1\n-- disposition-schema = disposition/v1\n-- generation = 1\n-- members = metal;eval\n-- rows = eval:not-applicable\ndef gap_surface(x: f32) -> f32 = x\n\
         -- chelis:provenance/v1 debt\n-- id = GAP-DEBT\n-- obligations = GAP-POLICY:freshness:atom:CHELIS-UNCOVERED@{digest}\n-- objects = atom:CHELIS-UNCOVERED@{digest}\n-- rationale = Evidence requires repair\n-- owner = chelis-core\n-- remediation = Replace stale evidence\n-- discharge-schema = discharge/v1\n-- discharge = Evidence is current\ndef gap_debt(x: f32) -> f32 = x\n\
         -- chelis:provenance/v1 waiver\n-- id = GAP-WAIVER\n-- obligations = GAP-POLICY:freshness:atom:CHELIS-UNCOVERED@{digest}\n-- objects = atom:CHELIS-UNCOVERED@{digest}\n-- rationale = Evidence requires review\n-- owner = chelis-core\n-- discharge-schema = discharge/v1\n-- discharge = Evidence is current\n-- approval-authority = chelis-core\n-- approval-record = reviews/waiver.md\n-- approval-digest = {digest}\ndef gap_waiver(x: f32) -> f32 = x\n"
    )
    .expect("gap records");
    let rich = ChelisSnapshot::from_files(vec![
        (
            String::from("chelis/gaps.ch"),
            ChelisDialect::Surf,
            rich_source.into_bytes(),
        ),
        (
            String::from("chelis/broken.ch"),
            ChelisDialect::Surf,
            b"def broken( = {\n".to_vec(),
        ),
        (
            String::from("chelis/invalid.ch"),
            ChelisDialect::Surf,
            vec![0xff],
        ),
    ])
    .expect("gap snapshot");
    let rich_outcome = static_check(&rich, &configuration).expect("gap outcome");
    let value: serde_json::Value =
        serde_json::from_str(&advisory_report(&rich, &rich_outcome).expect("gap report"))
            .expect("gap JSON");
    assert_eq!(value["uncovered_atoms"], serde_json::json!([]));
    assert_eq!(value["unatomized_normative_lines"], serde_json::json!([]));
    assert_eq!(
        value["undecided_surface_members"],
        serde_json::json!(["GAP-SURFACE:metal"])
    );
    assert_eq!(value["active_debt"], serde_json::json!(["GAP-DEBT"]));
    assert_eq!(value["active_waivers"], serde_json::json!(["GAP-WAIVER"]));
    assert_eq!(
        value["carrier_counts"],
        serde_json::json!({
            "positive": 1, "negative": 1, "property": 1, "exhaustive-model": 1,
            "formal-property": 1, "structural": 1,
        })
    );
}
